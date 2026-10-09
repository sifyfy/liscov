/**
 * 自動スクロールと最新への自動復帰 (docs/specs/02_chat.md「配信中の画面」の「自動スクロール」)
 *
 * テストの例は仕様の表から取る:
 * - 手でスクロールして最新から離れた → 自動スクロールが止まる
 * - 最新から離れたまま 30 秒、流れの上で操作が無い → 最新に戻り、自動スクロールを再開する
 * - 手で最新までスクロールした → 自動スクロールを再開する
 * - 「最新へ」のボタン: 残り時間を表す。流れの上で操作すると満ちた状態からやり直す
 */
import { describe, expect, it } from 'vitest';
import { AUTO_RETURN_MS, FOLLOWING, activity, leaveLatest, reachLatest, remainingMs, tick } from './auto-return';

const T0 = 1_000_000;

describe('自動スクロールと最新への自動復帰', () => {
  it('はじめは最新を追っている', () => {
    expect(FOLLOWING.following).toBe(true);
    expect(remainingMs(FOLLOWING, T0)).toBeNull();
  });

  it('手でスクロールして最新から離れたら、追うのをやめて 30 秒後に戻る予定にする', () => {
    const away = leaveLatest(FOLLOWING, T0);
    expect(away.following).toBe(false);
    expect(remainingMs(away, T0)).toBe(AUTO_RETURN_MS);
    expect(AUTO_RETURN_MS).toBe(30_000);
  });

  it('離れたまま 30 秒操作が無ければ最新に戻る。29 秒ではまだ戻らない', () => {
    const away = leaveLatest(FOLLOWING, T0);
    expect(tick(away, T0 + 29_000).following).toBe(false);
    expect(tick(away, T0 + 30_000)).toEqual(FOLLOWING);
  });

  it('流れの上で操作すると、そこから 30 秒に延びる (満ちた状態からやり直す)', () => {
    const away = leaveLatest(FOLLOWING, T0);
    const touched = activity(away, T0 + 20_000);
    expect(remainingMs(touched, T0 + 20_000)).toBe(AUTO_RETURN_MS);
    expect(tick(touched, T0 + 30_000).following).toBe(false);
    expect(tick(touched, T0 + 50_000)).toEqual(FOLLOWING);
  });

  it('手で最新までスクロールしたら、追うのを再開する', () => {
    expect(reachLatest()).toEqual(FOLLOWING);
  });

  it('追っている間の操作・時間の経過では何も変わらない', () => {
    expect(activity(FOLLOWING, T0)).toBe(FOLLOWING);
    expect(tick(FOLLOWING, T0 + 60_000)).toBe(FOLLOWING);
  });

  it('離れている間にまた離れても (続けてスクロール)、期限はそこから 30 秒', () => {
    const away = leaveLatest(FOLLOWING, T0);
    expect(remainingMs(leaveLatest(away, T0 + 10_000), T0 + 10_000)).toBe(AUTO_RETURN_MS);
  });

  it('残り時間は 0 より小さくならない', () => {
    expect(remainingMs(leaveLatest(FOLLOWING, T0), T0 + 40_000)).toBe(0);
  });
});
