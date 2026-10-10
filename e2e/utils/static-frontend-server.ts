/**
 * E2E 用の静的フロントエンドサーバー
 *
 * debug ビルドの exe はフロントを埋め込まず devUrl (http://localhost:5173) を読む。
 * このサーバーは空きポートでこのワークツリーの build/ を配信し、WebView2 の --host-resolver-rules で
 * devUrl の接続先をここへ付け替える (e2e/README.md「並列実行と分離」、ADR-005)。
 */

import { randomUUID } from 'crypto';
import * as fs from 'fs';
import * as http from 'http';
import type { AddressInfo } from 'net';
import * as path from 'path';

/**
 * 起動ごとの ID を返すパス。ページからこれを fetch して、付け替えが効いて自分のサーバーから読み込まれたかを確かめる
 */
export const FRONTEND_SERVER_ID_PATH = '/__liscov_e2e_server_id';

export interface StaticFrontendServer {
  readonly port: number;
  readonly serverId: string;
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

/**
 * rootDir を 127.0.0.1 の空きポートで配信する。ポートは OS が選ぶので、別のワークツリーの E2E や pnpm dev とぶつからない
 */
export async function startStaticFrontendServer(rootDir: string): Promise<StaticFrontendServer> {
  const serverId = randomUUID();
  const server = http.createServer((req, res) => {
    if (new URL(req.url ?? '/', 'http://127.0.0.1').pathname === FRONTEND_SERVER_ID_PATH) {
      res.writeHead(200, { 'Content-Type': 'text/plain; charset=utf-8', 'Cache-Control': 'no-store' });
      res.end(serverId);
      return;
    }
    const filePath = resolveStaticFrontendFile(rootDir, req.url);
    try {
      const content = fs.readFileSync(filePath);
      res.writeHead(200, { 'Content-Type': getStaticContentType(filePath) });
      res.end(content);
    } catch {
      res.writeHead(404);
      res.end('Not found');
    }
  });

  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => resolve());
  });
  const { port } = server.address() as AddressInfo;

  return { port, serverId, close: () => closeServer(server) };
}
