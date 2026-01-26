/**
 * E2Eテスト用ログユーティリティ
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
 */

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
    console.log(this.formatMessage(LogLevel.DEBUG, message) + formatData(data));
  }

  info(message: string): void {
    if (!this.shouldLog(LogLevel.INFO)) return;
    console.log(this.formatMessage(LogLevel.INFO, message));
  }

  warn(message: string): void {
    if (!this.shouldLog(LogLevel.WARN)) return;
    console.warn(this.formatMessage(LogLevel.WARN, message));
  }

  error(message: string, error?: Error): void {
    if (!this.shouldLog(LogLevel.ERROR)) return;
    const errorInfo = error ? `: ${error.message}` : '';
    console.error(this.formatMessage(LogLevel.ERROR, message + errorInfo));
  }

  child(prefix: string): TestLogger {
    const childPrefix = this.prefix ? `${this.prefix}:${prefix}` : prefix;
    const child = new TestLogger(childPrefix);
    // 親のレベル設定を継承（環境変数から再読み込みされる）
    return child;
  }
}

export const log = new TestLogger();
