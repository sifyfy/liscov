import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import LatestButton from './LatestButton.svelte';

// spec: docs/specs/02_chat.md「配信中の画面」の「自動スクロール」
// - 最新から離れているとき、流れの右下に「最新へ」のボタン。押すと最新に戻る
// - 自動で戻るまでの残り時間を帯で表す。流れの上で操作すると満ちた状態からやり直す
// - 動きを減らす設定では残りの秒数を文字で出す
// - 離れている間に届いた優先順位 1・2 の件数を添える (例: 「最新へ スパチャ等 2・新規さん 1」。0 件の種類は出さない)
// - 視聴者情報パネルで打っている間は戻らない (残り時間は出さない)
const NONE = { supporters: 0, newcomers: 0 };
const T0 = 1_000_000;

describe('LatestButton', () => {
  it('「最新へ」を押すと onClick を呼ぶ', async () => {
    const onClick = vi.fn();
    const { getByRole } = render(LatestButton, { props: { deadline: T0 + 30_000, now: T0, unread: NONE, onClick } });
    await fireEvent.click(getByRole('button', { name: /最新へ/ }));
    expect(onClick).toHaveBeenCalledOnce();
  });

  it('残りの秒数を文字で持つ (動きを減らす設定で見せる)', async () => {
    const { getByTestId, rerender } = render(LatestButton, { props: { deadline: T0 + 30_000, now: T0, unread: NONE, onClick: () => {} } });
    await rerender({ deadline: T0 + 30_000, now: T0 + 17_700, unread: NONE, onClick: () => {} });
    expect(getByTestId('latest-remaining').textContent).toBe('あと 13 秒で戻ります');
  });

  it('残り時間の帯は、残りの割合から 0 まで残りの時間をかけて縮む', () => {
    const { getByTestId } = render(LatestButton, { props: { deadline: T0 + 15_000, now: T0, unread: NONE, onClick: () => {} } });
    const bar = getByTestId('latest-bar');
    expect(bar.style.getPropertyValue('--from')).toBe('0.5');
    expect(bar.style.animationDuration).toBe('15000ms');
  });

  it('時間が進んでも帯は作り直さない (縮む動きが跳ばない)', async () => {
    const { getByTestId, rerender } = render(LatestButton, { props: { deadline: T0 + 30_000, now: T0, unread: NONE, onClick: () => {} } });
    const bar = getByTestId('latest-bar');
    await rerender({ deadline: T0 + 30_000, now: T0 + 5_000, unread: NONE, onClick: () => {} });
    expect(getByTestId('latest-bar')).toBe(bar);
    expect(bar.style.animationDuration).toBe('30000ms');
  });

  it('期限が延びたら帯を満ちた状態から作り直す。ボタンそのものは作り直さない (押している途中に消えない)', async () => {
    const { getByTestId, getByRole, rerender } = render(LatestButton, {
      props: { deadline: T0 + 30_000, now: T0, unread: NONE, onClick: () => {} },
    });
    const button = getByRole('button', { name: /最新へ/ });
    const bar = getByTestId('latest-bar');
    await rerender({ deadline: T0 + 40_000, now: T0 + 10_000, unread: NONE, onClick: () => {} });
    expect(getByRole('button', { name: /最新へ/ })).toBe(button);
    expect(getByTestId('latest-bar')).not.toBe(bar);
    expect(getByTestId('latest-bar').style.getPropertyValue('--from')).toBe('1');
  });

  it('離れている間に届いたスパチャ等・新規さんの件数を添える。0 件の種類は出さない', async () => {
    const { getByRole, rerender } = render(LatestButton, {
      props: { deadline: T0 + 30_000, now: T0, unread: { supporters: 2, newcomers: 1 }, onClick: () => {} },
    });
    expect(getByRole('button').textContent).toContain('最新へ スパチャ等 2・新規さん 1');
    await rerender({ deadline: T0 + 30_000, now: T0, unread: { supporters: 0, newcomers: 1 }, onClick: () => {} });
    expect(getByRole('button').textContent).toContain('最新へ 新規さん 1');
    expect(getByRole('button').textContent).not.toContain('スパチャ等');
    await rerender({ deadline: T0 + 30_000, now: T0, unread: NONE, onClick: () => {} });
    expect(getByRole('button', { name: /最新へ/ }).textContent).not.toMatch(/スパチャ等|新規さん/);
  });

  it('打っていて戻らない間 (期限なし) は、残り時間の帯も秒数も出さない', () => {
    const { queryByTestId } = render(LatestButton, { props: { deadline: null, now: T0, unread: NONE, onClick: () => {} } });
    expect(queryByTestId('latest-bar')).toBeNull();
    expect(queryByTestId('latest-remaining')).toBeNull();
  });
});
