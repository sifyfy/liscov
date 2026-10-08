// @vitest-environment node
/**
 * E2E をワークツリーごとに分離する仕組みの単体テスト
 *
 * 仕様: e2e/README.md「並列実行と分離」(ADR-005)。テストケースはそこに挙げた具体例から作っている。
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import {
  acquireWorktreeLock,
  describeFrontendServerMismatch,
  parseDevToolsActivePort,
  parseMockServerUrl,
  releaseWorktreeLock,
  webView2BrowserArguments,
  worktreeTestName,
} from './e2e-isolation';

describe('worktreeTestName', () => {
  it('liscov-test- に 16 進の小文字 8 桁を付けた名前にする', () => {
    expect(worktreeTestName('C:\\Users\\cat\\dev\\liscov', 'win32')).toMatch(/^liscov-test-[0-9a-f]{8}$/);
  });

  it('Windows ではパスの大文字・小文字が違っても同じ名前にする', () => {
    expect(worktreeTestName('c:\\users\\cat\\dev\\liscov', 'win32')).toBe(
      worktreeTestName('C:\\Users\\cat\\dev\\liscov', 'win32')
    );
  });

  it('別のワークツリーには別の名前を付ける', () => {
    expect(worktreeTestName('C:\\Users\\cat\\dev\\liscov\\.claude\\worktrees\\foo', 'win32')).not.toBe(
      worktreeTestName('C:\\Users\\cat\\dev\\liscov', 'win32')
    );
  });

  it('同じパスなら何度呼んでも同じ名前にする (実行ごとには変わらない)', () => {
    expect(worktreeTestName('C:\\Users\\cat\\dev\\liscov', 'win32')).toBe(
      worktreeTestName('C:\\Users\\cat\\dev\\liscov', 'win32')
    );
  });
});

describe('parseDevToolsActivePort', () => {
  it('1 行目のポートと 2 行目のパスから CDP のエンドポイントを作る', () => {
    expect(parseDevToolsActivePort('56032\n/devtools/browser/793d0980-164f-4b16-8e77-89cc5677dce5')).toEqual({
      port: 56032,
      browserWsUrl: 'ws://127.0.0.1:56032/devtools/browser/793d0980-164f-4b16-8e77-89cc5677dce5',
    });
  });

  it('CRLF の改行でも読める', () => {
    expect(parseDevToolsActivePort('56032\r\n/devtools/browser/abc\r\n')?.port).toBe(56032);
  });

  it.each([
    ['空 (書き始め)', ''],
    ['ポートだけ (書きかけ)', '56032\n'],
    ['1 行目が数字でない', 'abc\n/devtools/browser/abc'],
    ['ポートが 0', '0\n/devtools/browser/abc'],
    ['ポートが 65535 を超える', '65536\n/devtools/browser/abc'],
    ['2 行目が /devtools/browser/ で始まらない', '56032\n/json/version'],
  ])('%s なら、まだ書き終わっていないものとして null を返す', (_label, content) => {
    expect(parseDevToolsActivePort(content)).toBeNull();
  });
});

describe('parseMockServerUrl', () => {
  it('標準出力の Mock server on の行から URL を読む', () => {
    expect(parseMockServerUrl('Mock server on http://127.0.0.1:51234\n')).toBe('http://127.0.0.1:51234');
  });

  it('前後にほかの出力があっても読む', () => {
    expect(parseMockServerUrl('Loaded 3 entries\nMock server on http://127.0.0.1:51234\r\nGET /status\n')).toBe(
      'http://127.0.0.1:51234'
    );
  });

  it.each([
    ['まだ出力されていない', 'Loaded 3 entries\n'],
    ['行の途中までしか届いていない', 'Mock server on http://127.0.0.1:512'],
    ['ポートが 0 (bind 前のアドレス)', 'Mock server on http://127.0.0.1:0\n'],
  ])('%s なら null を返す', (_label, output) => {
    expect(parseMockServerUrl(output)).toBeNull();
  });
});

describe('webView2BrowserArguments', () => {
  it('CDP を空きポートで開き、devUrl の接続先を静的サーバーに付け替える', () => {
    expect(webView2BrowserArguments('http://localhost:5173', 56031)).toBe(
      '--remote-debugging-port=0 --host-resolver-rules="MAP localhost:5173 127.0.0.1:56031"'
    );
  });
});

describe('describeFrontendServerMismatch', () => {
  const expectedId = '0f8fad5b-d9cb-469f-a165-70867728950e';

  it('静的サーバーの ID と一致すれば null (続ける)', () => {
    expect(describeFrontendServerMismatch({ kind: 'fetched', body: expectedId }, expectedId)).toBeNull();
  });

  it('一致しなければ (例: Vite が index.html を返した) 止める理由を返す', () => {
    const reason = describeFrontendServerMismatch({ kind: 'fetched', body: '<!doctype html>' }, expectedId);
    expect(reason).toContain('フロントエンドが E2E の静的サーバーから読み込まれていません');
  });

  it('fetch が失敗したら (例: 5173 に誰もいない) 同じ理由で止める', () => {
    const reason = describeFrontendServerMismatch({ kind: 'failed', error: 'Failed to fetch' }, expectedId);
    expect(reason).toContain('フロントエンドが E2E の静的サーバーから読み込まれていません');
    expect(reason).toContain('Failed to fetch');
  });
});

describe('acquireWorktreeLock / releaseWorktreeLock', () => {
  let dir: string;
  let lockPath: string;
  const alive = (pids: readonly number[]) => (pid: number) => pids.includes(pid);

  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'liscov-e2e-lock-'));
    // .tmp がまだ無いワークツリーでも取れること
    lockPath = path.join(dir, '.tmp', 'e2e.lock');
  });

  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
  });

  it('ロックファイルが無ければ、自分のプロセス ID を書いて続ける', () => {
    acquireWorktreeLock(lockPath, 100, alive([100]));
    expect(fs.readFileSync(lockPath, 'utf8')).toBe('100');
  });

  it('自分のプロセス ID が書かれていれば続ける (同じ実行の中で何度起動してもよい)', () => {
    acquireWorktreeLock(lockPath, 100, alive([100]));
    expect(() => acquireWorktreeLock(lockPath, 100, alive([100]))).not.toThrow();
  });

  it('生きている別のプロセスの ID が書かれていれば、その PID を示して止める', () => {
    acquireWorktreeLock(lockPath, 200, alive([200]));
    expect(() => acquireWorktreeLock(lockPath, 100, alive([100, 200]))).toThrow(
      'このワークツリーでは別の E2E (PID 200) が実行中です'
    );
    expect(fs.readFileSync(lockPath, 'utf8')).toBe('200');
  });

  it('もう居ないプロセスの ID が書かれていれば、上書きして続ける', () => {
    acquireWorktreeLock(lockPath, 200, alive([200]));
    acquireWorktreeLock(lockPath, 100, alive([100]));
    expect(fs.readFileSync(lockPath, 'utf8')).toBe('100');
  });

  it('解放すると消え、別のプロセスが取れる', () => {
    acquireWorktreeLock(lockPath, 100, alive([100]));
    releaseWorktreeLock(lockPath, 100);
    expect(fs.existsSync(lockPath)).toBe(false);
    expect(() => acquireWorktreeLock(lockPath, 200, alive([100, 200]))).not.toThrow();
  });

  it('別のプロセスのロックは解放しない', () => {
    acquireWorktreeLock(lockPath, 200, alive([200]));
    releaseWorktreeLock(lockPath, 100);
    expect(fs.readFileSync(lockPath, 'utf8')).toBe('200');
  });
});
