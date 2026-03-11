/**
 * E2Eテスト用ログユーティリティ（リングバッファモード）
 *
 * デフォルトで全ログをリングバッファに蓄積し、コンソールには出力しない。
 * テスト失敗時にバッファをフラッシュして経緯を表示する。
 *
 * 使用例:
 *   import { log } from './utils/logger';
 *   log.debug('詳細情報', { data: value });
 *   log.info('接続完了');
 *   log.warn('タイムアウト接近');
 *   log.error('接続失敗', error);
 *
 * 環境変数:
 *   E2E_LOG_LEVEL=debug|info|warn|error|silent (デフォルト: info)
 *
 * バッファ制御（fixtures.ts + buffered-log-reporter.ts から使用）:
 *   drainBuffer() - バッファ内容を文字列として返しクリア
 *   clearBuffer() - バッファを破棄
 *   flushBuffer() - バッファ内容を stderr に同期書き込み（process.on('exit') 用）
 */

import * as fs from 'fs';

export enum LogLevel {
  DEBUG = 0,
  INFO = 1,
  WARN = 2,
  ERROR = 3,
  SILENT = 4,
}

const LEVEL_NAMES: Record<LogLevel, string> = {
  [LogLevel.DEBUG]: 'DEBUG',
  [LogLevel.INFO]: 'INFO',
  [LogLevel.WARN]: 'WARN',
  [LogLevel.ERROR]: 'ERROR',
  [LogLevel.SILENT]: 'SILENT',
};

function parseLevel(level: string): LogLevel {
  switch (level.toLowerCase()) {
    case 'debug':
      return LogLevel.DEBUG;
    case 'info':
      return LogLevel.INFO;
    case 'warn':
      return LogLevel.WARN;
    case 'error':
      return LogLevel.ERROR;
    case 'silent':
      return LogLevel.SILENT;
    default:
      return LogLevel.INFO;
  }
}

function formatData(data: unknown): string {
  if (data === undefined) return '';
  try {
    return ' ' + JSON.stringify(data);
  } catch {
    return ' [unserializable]';
  }
}

// --- リングバッファ ---
// 注意: この設計は workers=1（シリアル実行）を前提とする。
// 複数ワーカーで実行すると共有バッファへの書き込みが競合する。

const MAX_BUFFER_LINES = 200;
let ringBuffer: string[] = [];
let truncatedCount = 0;

function bufferLog(message: string): void {
  if (ringBuffer.length >= MAX_BUFFER_LINES) {
    ringBuffer.shift();
    truncatedCount++;
  }
  ringBuffer.push(message);
}

/**
 * バッファ内容を文字列として返しクリアする。
 * fixtures.ts から testInfo.attach() 経由でレポーターに渡す用途。
 */
export function drainBuffer(): string {
  if (ringBuffer.length === 0) return '';

  const lines: string[] = [];
  if (truncatedCount > 0) {
    lines.push(`... ${truncatedCount} lines truncated ...`);
  }
  lines.push(...ringBuffer);

  ringBuffer = [];
  truncatedCount = 0;

  return lines.join('\n');
}

/**
 * バッファ内容を stderr に同期書き込みしてクリアする。
 * process.on('exit') での最終フラッシュ専用。
 */
export function flushBuffer(): void {
  const content = drainBuffer();
  if (!content) return;

  try {
    fs.writeSync(process.stderr.fd, content + '\n');
  } catch {
    // fd が閉じている等の異常時は無視
  }
}

/**
 * バッファを破棄する。成功テスト後のクリーンアップに使用。
 */
export function clearBuffer(): void {
  ringBuffer = [];
  truncatedCount = 0;
}

// プロセス異常終了時の最終フラッシュ
process.on('exit', () => {
  if (ringBuffer.length > 0) {
    flushBuffer();
  }
});

export class TestLogger {
  private readonly level: LogLevel;
  private readonly prefix: string;

  constructor(prefix = '') {
    this.level = parseLevel(process.env.E2E_LOG_LEVEL || 'info');
    this.prefix = prefix;
  }

  private shouldLog(level: LogLevel): boolean {
    return level >= this.level;
  }

  private formatMessage(level: LogLevel, message: string): string {
    const levelName = LEVEL_NAMES[level];
    return this.prefix
      ? `[${levelName}] [${this.prefix}] ${message}`
      : `[${levelName}] ${message}`;
  }

  debug(message: string, data?: unknown): void {
    if (!this.shouldLog(LogLevel.DEBUG)) return;
    bufferLog(this.formatMessage(LogLevel.DEBUG, message) + formatData(data));
  }

  info(message: string): void {
    if (!this.shouldLog(LogLevel.INFO)) return;
    bufferLog(this.formatMessage(LogLevel.INFO, message));
  }

  warn(message: string): void {
    if (!this.shouldLog(LogLevel.WARN)) return;
    bufferLog(this.formatMessage(LogLevel.WARN, message));
  }

  error(message: string, error?: Error): void {
    if (!this.shouldLog(LogLevel.ERROR)) return;
    const errorInfo = error ? `: ${error.message}` : '';
    bufferLog(this.formatMessage(LogLevel.ERROR, message + errorInfo));
  }

  child(prefix: string): TestLogger {
    const childPrefix = this.prefix ? `${this.prefix}:${prefix}` : prefix;
    // 子ロガーも同じ共有リングバッファに書き込む
    return new TestLogger(childPrefix);
  }
}

export const log = new TestLogger();
