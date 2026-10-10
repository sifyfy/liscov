import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import StatusLine from './StatusLine.svelte';

// spec: docs/specs/02_chat.md「配信中の画面」の「状態」と「状態の行の統計（StatusLine）」、
// 「ライブリアクション」の「状態の行のリアクション」
const base = {
  connected: true,
  count: 1820,
  perMinute: 48,
  viewers: 312,
  shownLabel: null,
  reactions: [],
  reactionsLabel: '',
  error: null,
  onDismissError: () => {},
  alerts: [],
  onAlert: () => {},
};

describe('StatusLine', () => {
  it('接続中: 点 (success)・件・/分・人を出す', () => {
    const { container } = render(StatusLine, { props: base });
    expect(container.textContent).toContain('1,820 件');
    expect(container.textContent).toContain('48/分');
    expect(container.textContent).toContain('312 人');
    expect(screen.getByTestId('status-dot').className).toContain('bg-success');
  });

  it('未接続: 控えめな点と「未接続」だけ。統計は出さない', () => {
    const { container } = render(StatusLine, { props: { ...base, connected: false } });
    expect(container.textContent).toContain('未接続');
    expect(container.textContent).not.toContain('件');
    expect(screen.getByTestId('status-dot').className).toContain('bg-fg-3');
  });

  it('一部を隠しているときは「表示中 N / 全 M 件」を出す', () => {
    const { container } = render(StatusLine, { props: { ...base, shownLabel: '表示中 50 / 全 80 件' } });
    expect(container.textContent).toContain('表示中 50 / 全 80 件');
  });

  it('主の配信のリアクションを右端に出し、全部をヒントに出す', () => {
    render(StatusLine, {
      props: {
        ...base,
        reactions: [
          ['❤', 430],
          ['🎉', 418],
          ['😄', 94],
        ],
        reactionsLabel: 'リアクション ❤ 430、🎉 418、😄 94、💯 56',
      },
    });
    const reactions = screen.getByTestId('status-reactions');
    expect([...reactions.children].map((c) => c.textContent)).toEqual(['❤ 430', '🎉 418', '😄 94']);
    expect(reactions.getAttribute('title')).toBe('リアクション ❤ 430、🎉 418、😄 94、💯 56');
    expect(reactions.getAttribute('aria-label')).toBe('リアクション ❤ 430、🎉 418、😄 94、💯 56');
  });

  it('エラーのときは error の面にして理由を出し、「閉じる」で消せる', async () => {
    const onDismissError = vi.fn();
    render(StatusLine, {
      props: { ...base, error: 'チャットの取得に15回続けて失敗したため切断しました', onDismissError },
    });
    const line = screen.getByRole('alert');
    expect(line.textContent).toContain('チャットの取得に15回続けて失敗したため切断しました');
    expect(line.className).toContain('bg-error');
    await fireEvent.click(screen.getByRole('button', { name: '閉じる' }));
    expect(onDismissError).toHaveBeenCalledOnce();
  });

  it('異常がなければ出さない', () => {
    render(StatusLine, { props: base });
    expect(screen.queryByTestId('status-alert')).toBeNull();
  });

  it('セッション切れ・WebSocket の停止: error の色の文字で出し、押すとその異常を知らせる。行は error の面にしない', async () => {
    const onAlert = vi.fn();
    const { container } = render(StatusLine, {
      props: { ...base, alerts: ['session_expired', 'websocket_stopped'], onAlert },
    });
    const alerts = screen.getAllByTestId('status-alert');
    expect(alerts.map((a) => a.textContent?.trim())).toEqual(['ログイン: セッション切れ', 'WebSocket: 止まっています']);
    expect(alerts.every((a) => a.className.includes('text-error'))).toBe(true);
    expect(container.querySelector('.bg-error')).toBeNull();
    // 直るまで出したまま (閉じるボタンは無い)
    expect(screen.queryByRole('button', { name: '閉じる' })).toBeNull();
    await fireEvent.click(alerts[0]);
    expect(onAlert).toHaveBeenCalledWith('session_expired');
    await fireEvent.click(alerts[1]);
    expect(onAlert).toHaveBeenCalledWith('websocket_stopped');
  });

  it('未接続でも異常は出す', () => {
    render(StatusLine, { props: { ...base, connected: false, alerts: ['storage_error'] } });
    expect(screen.getByTestId('status-alert').textContent?.trim()).toBe('ストレージエラー');
  });
});
