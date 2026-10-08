// @vitest-environment node
/**
 * E2E 用の静的フロントエンドサーバーの単体テスト
 *
 * 仕様 (e2e/README.md「並列実行と分離」、ADR-005):
 * - build/ を 127.0.0.1 の空きポートで配信する (WebView2 の localhost:5173 はそこへ付け替える)
 * - 起動ごとの ID を GET /__liscov_e2e_server_id で返す。ほかのパスは build/ のファイル、無ければ index.html
 */
import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { FRONTEND_SERVER_ID_PATH, startStaticFrontendServer, type StaticFrontendServer } from './static-frontend-server';

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

async function start(): Promise<StaticFrontendServer> {
  const server = await startStaticFrontendServer(rootDir);
  cleanups.push(() => server.close());
  return server;
}

async function get(port: number, urlPath: string): Promise<{ status: number; type: string; body: string }> {
  const response = await fetch(`http://127.0.0.1:${port}${urlPath}`);
  return { status: response.status, type: response.headers.get('content-type') ?? '', body: await response.text() };
}

describe('startStaticFrontendServer', () => {
  it('127.0.0.1 の空きポートで build/ を配信し、そのポートを返す', async () => {
    const { port } = await start();

    expect(port).toBeGreaterThan(0);
    expect(await get(port, '/')).toEqual({ status: 200, type: 'text/html; charset=utf-8', body: '<p>index</p>' });
    expect(await get(port, '/_app/app.js')).toMatchObject({ type: 'application/javascript; charset=utf-8', body: 'console.log(1)' });
  });

  it('同時に起動しても別のポートになる (別のワークツリーの E2E と並列に動く)', async () => {
    const [a, b] = await Promise.all([start(), start()]);

    expect(a.port).not.toBe(b.port);
  });

  it(`${FRONTEND_SERVER_ID_PATH} に起動ごとの ID を返す`, async () => {
    const [a, b] = await Promise.all([start(), start()]);

    expect(await get(a.port, FRONTEND_SERVER_ID_PATH)).toEqual({ status: 200, type: 'text/plain; charset=utf-8', body: a.serverId });
    expect(a.serverId).toMatch(/^[0-9a-f-]{36}$/);
    expect(a.serverId).not.toBe(b.serverId);
  });

  it('存在しないパスは index.html を返す (SPA のフォールバック)', async () => {
    const { port } = await start();

    expect((await get(port, '/settings')).body).toBe('<p>index</p>');
  });

  it('build/ の外を指すパスでも build/ の外のファイルは返さない', async () => {
    const { port } = await start();

    expect((await get(port, '/..%2F..%2Fwindows%2Fwin.ini')).body).toBe('<p>index</p>');
  });

  it('close() でポートを解放する', async () => {
    const server = await startStaticFrontendServer(rootDir);
    await server.close();

    await expect(fetch(`http://127.0.0.1:${server.port}/`)).rejects.toThrow();
  });
});
