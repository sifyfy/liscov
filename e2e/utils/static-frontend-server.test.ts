// @vitest-environment node
/**
 * E2E 用の静的フロントエンドサーバーの単体テスト
 *
 * 仕様 (e2e/README.md「ポート 5173 が使用中でエラーになる」):
 * - build/ を 127.0.0.1 と ::1 の両方の同じポートで配信する
 *   (WebView2 は localhost を ::1 から試すため、片方だけだと別のサーバーに読み込まれる)
 * - どちらかのアドレスが使用中なら再利用せずにエラーで止め、掴みかけたポートも解放する
 */
import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import * as fs from 'fs';
import * as http from 'http';
import * as os from 'os';
import * as path from 'path';
import { startStaticFrontendServer, type StaticFrontendServer } from './static-frontend-server';

let rootDir: string;
const cleanups: (() => Promise<void>)[] = [];

beforeAll(() => {
  rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'liscov-static-frontend-'));
  fs.writeFileSync(path.join(rootDir, 'index.html'), '<p>index</p>');
  fs.mkdirSync(path.join(rootDir, '_app'));
  fs.writeFileSync(path.join(rootDir, '_app', 'app.js'), 'console.log(1)');
  return () => fs.rmSync(rootDir, { recursive: true, force: true });
});

afterEach(async () => {
  for (const cleanup of cleanups.splice(0).reverse()) await cleanup();
});

function listen(server: http.Server, port: number, host: string): Promise<void> {
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, host, resolve);
  });
}

/** 127.0.0.1 と ::1 の両方で空いているポートを探す */
async function findFreePort(): Promise<number> {
  for (;;) {
    const v4 = http.createServer();
    await listen(v4, 0, '127.0.0.1');
    const { port } = v4.address() as { port: number };
    const v6 = http.createServer();
    const ok = await listen(v6, port, '::1').then(() => true, () => false);
    await new Promise((resolve) => v4.close(resolve));
    if (ok) await new Promise((resolve) => v6.close(resolve));
    if (ok) return port;
  }
}

/** 別のプロセス (pnpm dev の Vite や別ワークツリーの E2E) が先に掴んでいる状況を作る */
async function occupy(port: number, host: string): Promise<void> {
  const server = http.createServer((_req, res) => res.end('other server'));
  await listen(server, port, host);
  cleanups.push(() => new Promise((resolve) => server.close(() => resolve())));
}

async function start(port: number): Promise<StaticFrontendServer> {
  const server = await startStaticFrontendServer(rootDir, port);
  cleanups.push(() => server.close());
  return server;
}

async function get(host: string, port: number, urlPath: string): Promise<{ status: number; type: string; body: string }> {
  const url = `http://${host.includes(':') ? `[${host}]` : host}:${port}${urlPath}`;
  const response = await fetch(url);
  return { status: response.status, type: response.headers.get('content-type') ?? '', body: await response.text() };
}

describe('startStaticFrontendServer', () => {
  it('127.0.0.1 と ::1 の両方で build/ を配信する', async () => {
    const port = await findFreePort();
    await start(port);

    for (const host of ['127.0.0.1', '::1']) {
      expect(await get(host, port, '/')).toEqual({ status: 200, type: 'text/html; charset=utf-8', body: '<p>index</p>' });
      expect(await get(host, port, '/_app/app.js')).toMatchObject({ type: 'application/javascript; charset=utf-8', body: 'console.log(1)' });
    }
  });

  it('存在しないパスは index.html を返す (SPA のフォールバック)', async () => {
    const port = await findFreePort();
    await start(port);

    expect((await get('127.0.0.1', port, '/settings')).body).toBe('<p>index</p>');
  });

  it('build/ の外を指すパスでも build/ の外のファイルは返さない', async () => {
    const port = await findFreePort();
    await start(port);

    expect((await get('127.0.0.1', port, '/..%2F..%2Fwindows%2Fwin.ini')).body).toBe('<p>index</p>');
  });

  it.each(['::1', '127.0.0.1'])('%s が使用中なら、ポート番号と原因の候補を示して失敗する', async (busyHost) => {
    const port = await findFreePort();
    await occupy(port, busyHost);

    const error = await startStaticFrontendServer(rootDir, port).then(
      (server) => { cleanups.push(() => server.close()); return null; },
      (e: unknown) => e,
    );

    expect(error).toBeInstanceOf(Error);
    expect((error as Error).message).toContain(`${busyHost}:${port}`);
    expect((error as Error).message).toContain('pnpm dev');
  });

  it('片方だけ掴めた状態で失敗したら、掴んだ側も解放する', async () => {
    const port = await findFreePort();
    await occupy(port, '::1');

    await expect(startStaticFrontendServer(rootDir, port)).rejects.toThrow();

    // 127.0.0.1 側が解放されていれば、あらためて listen できる
    const probe = http.createServer();
    await listen(probe, port, '127.0.0.1');
    await new Promise((resolve) => probe.close(resolve));
  });

  it('close() で両方のアドレスを解放する', async () => {
    const port = await findFreePort();
    const server = await startStaticFrontendServer(rootDir, port);
    await server.close();

    const restarted = await start(port);
    expect(restarted).toBeDefined();
  });
});
