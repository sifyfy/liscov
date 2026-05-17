/**
 * TTS 機能を実 Tauri 経由で検証するための共通ヘルパー。
 *
 * - HTTP モック棒読みちゃんサーバー (動的ポート)
 * - tts_config.toml のテスト用書き込み
 * - speak リクエスト待機ユーティリティ
 */

import * as http from 'http';
import * as fs from 'fs';
import * as path from 'path';
import { getTestAppDataDir } from './test-helpers';

export interface MockBouyomichan {
  readonly server: http.Server;
  readonly port: number;
  /**
   * モック棒読みちゃンが /Talk で受信した text パラメータ。
   * 空文字 (接続テスト由来) は記録対象外。
   * テスト内では assertion 用 read-only のつもりで扱うが、内部で push する
   * 必要があるため readonly 修飾子は付けない。
   */
  readonly receivedTexts: string[];
}

export async function startMockBouyomichan(): Promise<MockBouyomichan> {
  const receivedTexts: string[] = [];
  const server = http.createServer((req, res) => {
    const url = new URL(req.url ?? '/', 'http://127.0.0.1');
    if (url.pathname === '/Talk') {
      const text = url.searchParams.get('text') ?? '';
      if (text !== '') {
        receivedTexts.push(text);
      }
    }
    res.writeHead(200);
    res.end();
  });
  const port = await new Promise<number>((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const addr = server.address();
      if (addr && typeof addr === 'object') {
        resolve(addr.port);
      } else {
        reject(new Error('Failed to resolve mock bouyomichan port'));
      }
    });
  });
  return { server, port, receivedTexts };
}

export async function stopMockBouyomichan(mock: MockBouyomichan): Promise<void> {
  await new Promise<void>((resolve) => mock.server.close(() => resolve()));
}

export interface TtsConfigOptions {
  readonly bouyomichanPort: number;
  readonly firstCommentPrefixEnabled?: boolean;
  readonly firstCommentPrefix?: string;
  readonly firstCommentOnly?: boolean;
}

/**
 * テスト用 LISCOV_APP_NAME 配下に tts_config.toml を生成する。
 * Tauri 起動時に TtsConfig::load() が読み込み、テストごとの設定で起動する。
 */
export function writeTestTtsConfig(options: TtsConfigOptions): void {
  const configDir = getTestAppDataDir();
  fs.mkdirSync(configDir, { recursive: true });
  const configPath = path.join(configDir, 'tts_config.toml');
  const prefixEnabled = options.firstCommentPrefixEnabled ?? false;
  const prefix = options.firstCommentPrefix ?? '';
  const only = options.firstCommentOnly ?? false;
  const toml = `enabled = true
backend = "bouyomichan"
read_author_name = true
add_honorific = true
strip_at_prefix = true
strip_handle_suffix = true
read_superchat_amount = true
max_text_length = 200
queue_size_limit = 50
first_comment_prefix_enabled = ${prefixEnabled}
first_comment_prefix = "${prefix}"
first_comment_only = ${only}

[bouyomichan]
host = "127.0.0.1"
port = ${options.bouyomichanPort}
voice = 0
volume = -1
speed = -1
tone = -1
auto_launch = false
auto_close = true

[voicevox]
host = "127.0.0.1"
port = 50021
speaker_id = 1
volume_scale = 1.0
speed_scale = 1.0
pitch_scale = 0.0
intonation_scale = 1.0
auto_launch = false
auto_close = true
`;
  fs.writeFileSync(configPath, toml, 'utf-8');
}

/**
 * モック棒読みちゃんが期待数の text を受信するまで待機する。
 * 失敗時は受信内容を含む詳細メッセージで throw する。
 */
export async function waitForReceivedTexts(
  mock: MockBouyomichan,
  expected: number,
  timeoutMs: number,
): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (mock.receivedTexts.length >= expected) return;
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(
    `Mock bouyomichan received ${mock.receivedTexts.length} text(s), expected >= ${expected}. ` +
      `received: ${JSON.stringify(mock.receivedTexts)}`,
  );
}

/**
 * 「N 件来てから T ミリ秒待っても増えない」ことを確認する。
 * "speak されないこと" を検証したい場合に使う。
 */
export async function assertNoFurtherSpeak(
  mock: MockBouyomichan,
  expectedCount: number,
  quietPeriodMs: number,
): Promise<void> {
  await new Promise((r) => setTimeout(r, quietPeriodMs));
  if (mock.receivedTexts.length !== expectedCount) {
    throw new Error(
      `Expected exactly ${expectedCount} speak call(s) after ${quietPeriodMs}ms quiet period, ` +
        `but observed ${mock.receivedTexts.length}. received: ${JSON.stringify(mock.receivedTexts)}`,
    );
  }
}
