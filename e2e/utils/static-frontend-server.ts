/**
 * E2E 用の静的フロントエンドサーバー
 *
 * debug ビルドの exe はフロントを埋め込まず devUrl (http://localhost:5173) を読む。
 * そこへこのワークツリーの build/ を配信するのがこのサーバーの役割。
 */

import * as fs from 'fs';
import * as http from 'http';
import * as path from 'path';

/**
 * localhost の両方のループバックで listen する。
 * WebView2 (Chromium) は localhost を ::1 から試すので、127.0.0.1 だけだと
 * ::1 で待つ pnpm dev の Vite (host 未指定時は ::1 に bind する) に黙って読み込まれる。
 */
const LOOPBACK_HOSTS = ['127.0.0.1', '::1'] as const satisfies readonly string[];

export interface StaticFrontendServer {
  readonly close: () => Promise<void>;
}

/**
 * レスポンスコンテンツタイプを拡張子から解決する
 */
function getStaticContentType(filePath: string): string {
  switch (path.extname(filePath).toLowerCase()) {
    case '.css':
      return 'text/css; charset=utf-8';
    case '.html':
      return 'text/html; charset=utf-8';
    case '.ico':
      return 'image/x-icon';
    case '.js':
      return 'application/javascript; charset=utf-8';
    case '.json':
      return 'application/json; charset=utf-8';
    case '.png':
      return 'image/png';
    case '.svg':
      return 'image/svg+xml';
    case '.txt':
      return 'text/plain; charset=utf-8';
    case '.woff2':
      return 'font/woff2';
    default:
      return 'application/octet-stream';
  }
}

/**
 * リクエストURLからビルド済みフロントエンドのファイルパスを解決する（該当なしは SPA として index.html）
 */
function resolveStaticFrontendFile(rootDir: string, requestUrl?: string): string {
  const requestPath = decodeURIComponent(new URL(requestUrl ?? '/', 'http://127.0.0.1').pathname);
  const relativePath = requestPath === '/' ? 'index.html' : requestPath.replace(/^\/+/, '');
  const root = path.resolve(rootDir);
  const resolvedPath = path.resolve(root, relativePath);

  if (resolvedPath.startsWith(root) && fs.existsSync(resolvedPath) && fs.statSync(resolvedPath).isFile()) {
    return resolvedPath;
  }

  return path.join(root, 'index.html');
}

function closeServer(server: http.Server): Promise<void> {
  return new Promise((resolve) => {
    // WebView2 の keep-alive 接続が残っていても閉じ切れるようにする
    server.closeAllConnections();
    server.close(() => resolve());
  });
}

function listenOn(server: http.Server, port: number, host: string): Promise<void> {
  return new Promise((resolve, reject) => {
    server.once('error', (error: NodeJS.ErrnoException) => {
      if (error.code !== 'EADDRINUSE') {
        reject(error);
        return;
      }
      // 再利用はしない: 先客が別ワークツリーや pnpm dev だと、このワークツリーの build/ ではないフロントを検証してしまう
      reject(
        new Error(
          `E2E のフロントエンド配信用のポート ${host}:${port} が使用中です。` +
            '`pnpm dev` / `pnpm tauri dev` の Vite や、別のワークツリーの E2E が動いていないか確認し、止めてから再実行してください。' +
            `使用中のプロセスは \`Get-NetTCPConnection -LocalPort ${port} -State Listen\` の OwningProcess で確認できます。`,
          { cause: error }
        )
      );
    });
    server.listen(port, host, () => resolve());
  });
}

/**
 * rootDir を 127.0.0.1 と ::1 の両方の port で配信する。どちらかが使用中なら両方とも閉じて失敗する
 */
export async function startStaticFrontendServer(rootDir: string, port: number): Promise<StaticFrontendServer> {
  const handler: http.RequestListener = (req, res) => {
    const filePath = resolveStaticFrontendFile(rootDir, req.url);
    try {
      const content = fs.readFileSync(filePath);
      res.writeHead(200, { 'Content-Type': getStaticContentType(filePath) });
      res.end(content);
    } catch {
      res.writeHead(404);
      res.end('Not found');
    }
  };

  const servers = LOOPBACK_HOSTS.map(() => http.createServer(handler));
  const close = async () => {
    await Promise.all(servers.filter((server) => server.listening).map(closeServer));
  };

  const results = await Promise.allSettled(servers.map((server, i) => listenOn(server, port, LOOPBACK_HOSTS[i])));
  const failure = results.find((result): result is PromiseRejectedResult => result.status === 'rejected');
  if (failure) {
    await close();
    throw failure.reason;
  }

  return { close };
}
