/**
 * バッファログ出力用カスタムレポーター
 *
 * fixtures.ts の autoフィクスチャが testInfo.attach() で保存したログを、
 * テスト結果行の後に出力する。
 *
 * playwright.config.ts で list レポーターの後に配置することで、
 * ✓/✘ 行の後にログが表示される：
 *
 *   reporter: [['list'], ['./e2e/utils/buffered-log-reporter.ts']]
 */

import type { Reporter, TestCase, TestResult } from '@playwright/test/reporter';
import { LOG_ATTACHMENT_NAME } from './fixtures';

export default class BufferedLogReporter implements Reporter {
  onTestEnd(_test: TestCase, result: TestResult): void {
    const attachment = result.attachments.find((a) => a.name === LOG_ATTACHMENT_NAME);
    if (!attachment?.body) return;

    const content = attachment.body.toString('utf8');
    if (!content) return;

    // 各行にインデントを付けて出力
    const indented = content
      .split('\n')
      .map((line) => `    ${line}`)
      .join('\n');
    process.stdout.write(indented + '\n');
  }
}
