/**
 * E2E をワークツリーごとに分離するための判定 (仕様: e2e/README.md「並列実行と分離」、ADR-005)
 *
 * プロセスやページに触れない純粋な判定だけを置き、test-helpers.ts から呼ぶ。
 */

import { createHash } from 'crypto';
import * as fs from 'fs';
import * as path from 'path';

/**
 * ワークツリーごとのテスト用の名前 (`liscov-test-<パスの SHA-256 先頭 8 桁>`)
 *
 * 実行ごとに変えないのは、落ちた実行の残骸 (データ・資格情報) を次の実行が掃除できるようにするため。
 */
export function worktreeTestName(projectDir: string, platform: NodeJS.Platform = process.platform): string {
  const resolved = path.resolve(projectDir);
  // Windows のパスは大文字・小文字を区別しないので、表記の揺れで別の名前にならないようにそろえる
  const normalized = platform === 'win32' ? resolved.toLowerCase() : resolved;
  return `liscov-test-${createHash('sha256').update(normalized).digest('hex').slice(0, 8)}`;
}

export interface DevToolsEndpoint {
  readonly port: number;
  readonly browserWsUrl: string;
}

function parsePort(text: string): number | null {
  if (!/^\d+$/.test(text)) return null;
  const port = Number(text);
  return port >= 1 && port <= 65535 ? port : null;
}

/**
 * WebView2 が書く DevToolsActivePort (1 行目: ポート、2 行目: ブラウザのパス) を読む。
 * 書きかけなど読めないときは null (呼び出し側は待ち続ける)
 */
export function parseDevToolsActivePort(content: string): DevToolsEndpoint | null {
  const [portLine = '', pathLine = ''] = content.split(/\r?\n/);
  const port = parsePort(portLine);
  if (port === null || !pathLine.startsWith('/devtools/browser/')) return null;
  return { port, browserWsUrl: `ws://127.0.0.1:${port}${pathLine}` };
}

/**
 * mock-server の標準出力から、実際に bind した URL を読む。改行まで届いていない行は読まない
 */
export function parseMockServerUrl(output: string): string | null {
  for (const match of output.matchAll(/^Mock server on (http:\/\/127\.0\.0\.1:(\d+))\r?\n/gm)) {
    if (parsePort(match[2]) !== null) return match[1];
  }
  return null;
}

/**
 * WebView2 の追加引数: CDP を空きポートで開き、devUrl の接続先だけを静的サーバーへ付け替える
 * (ページの URL は devUrl のままなので、Tauri の IPC のオリジン判定は変わらない)
 */
export function webView2BrowserArguments(devUrl: string, frontendPort: number): string {
  const { host } = new URL(devUrl);
  return `--remote-debugging-port=0 --host-resolver-rules="MAP ${host} 127.0.0.1:${frontendPort}"`;
}

/** ページから FRONTEND_SERVER_ID_PATH を fetch した結果 */
export type FrontendServerIdFetch =
  | { readonly kind: 'fetched'; readonly body: string }
  | { readonly kind: 'failed'; readonly error: string };

function frontendServerProblem(result: FrontendServerIdFetch, expectedId: string): string | null {
  switch (result.kind) {
    case 'fetched':
      return result.body === expectedId ? null : `ID が一致しません (応答の先頭: ${JSON.stringify(result.body.slice(0, 80))})`;
    case 'failed':
      return `ID を取得できません (${result.error})`;
    default: {
      const exhaustive: never = result;
      return exhaustive;
    }
  }
}

/**
 * ページが E2E の静的サーバーから読み込まれたかを判定する。問題なければ null、あれば止める理由
 */
export function describeFrontendServerMismatch(result: FrontendServerIdFetch, expectedId: string): string | null {
  const problem = frontendServerProblem(result, expectedId);
  if (problem === null) return null;
  return (
    `フロントエンドが E2E の静的サーバーから読み込まれていません: ${problem}。` +
    'WebView2 が --host-resolver-rules に従わず、本物の devUrl を読んだ可能性があります (e2e/README.md のトラブルシューティング)。'
  );
}

/** プロセスが生きているか (EPERM は「居るが権限が無い」なので生きている扱い) */
export function isProcessAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return (error as NodeJS.ErrnoException).code === 'EPERM';
  }
}

/**
 * ワークツリーのロックを取る。同じワークツリーの別の E2E が動いていれば例外で止める
 */
export function acquireWorktreeLock(lockPath: string, pid: number, isAlive: (pid: number) => boolean = isProcessAlive): void {
  fs.mkdirSync(path.dirname(lockPath), { recursive: true });
  try {
    fs.writeFileSync(lockPath, String(pid), { flag: 'wx' });
    return;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'EEXIST') throw error;
  }

  const holder = Number(fs.readFileSync(lockPath, 'utf8').trim());
  if (holder === pid) return;
  if (Number.isInteger(holder) && holder > 0 && isAlive(holder)) {
    throw new Error(
      `このワークツリーでは別の E2E (PID ${holder}) が実行中です。終わるのを待つか、別のワークツリーで走らせてください。` +
        `その PID が E2E でなければ ${lockPath} を消して再実行してください。`
    );
  }
  // 前の実行が落ちて消せなかったロック
  fs.writeFileSync(lockPath, String(pid));
}

/** 自分のロックだけを解放する */
export function releaseWorktreeLock(lockPath: string, pid: number): void {
  try {
    if (fs.readFileSync(lockPath, 'utf8').trim() === String(pid)) fs.rmSync(lockPath, { force: true });
  } catch {
    // 既に無い
  }
}
