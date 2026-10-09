/**
 * 自動スクロールと最新への自動復帰 (docs/specs/02_chat.md「配信中の画面」の「自動スクロール」)
 *
 * テストの例は仕様の表から取る:
 * - 手でスクロールして最新から離れた → 自動スクロールが止まる
 * - 最新から離れたまま 30 秒、流れの上で操作が無い → 最新に戻り、自動スクロールを再開する
 * - 手で最新までスクロールした → 自動スクロールを再開する
 * - 「最新へ」のボタン: 残り時間を表す。流れの上で操作すると満ちた状態からやり直す
 * - 視聴者情報パネルで打っている (欄にフォーカスがある・保存していない変更がある) 間は 30 秒で戻らない。
 *   フォーカスが外れ、変更も無ければ、そこから 30 秒で戻る (「視聴者の読み仮名・メモ」)
 */
import { describe, expect, it } from 'vitest';
import { AUTO_RETURN_MS, FOLLOWING, activity, hold, leaveLatest, reachLatest, release, remainingMs, tick } from './auto-return';

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

  describe('視聴者情報パネルで打っている間', () => {
    it('離れている間に打ちはじめたら、どれだけ時間がたっても戻らない。残り時間は無い', () => {
      const held = hold(leaveLatest(FOLLOWING, T0));
      expect(held.following).toBe(false);
      expect(remainingMs(held, T0 + 1_000)).toBeNull();
      expect(tick(held, T0 + 600_000)).toBe(held);
    });

    it('打っている間に流れの上で操作・スクロールしても、戻らないまま', () => {
      const held = hold(leaveLatest(FOLLOWING, T0));
      expect(activity(held, T0 + 5_000)).toBe(held);
      expect(leaveLatest(held, T0 + 5_000)).toBe(held);
      expect(tick(leaveLatest(held, T0 + 5_000), T0 + 60_000).following).toBe(false);
    });

    it('打ち終えたら (フォーカスが外れ変更も無い)、そこから 30 秒で戻る', () => {
      const held = hold(leaveLatest(FOLLOWING, T0));
      const released = release(held, T0 + 100_000);
      expect(remainingMs(released, T0 + 100_000)).toBe(AUTO_RETURN_MS);
      expect(tick(released, T0 + 129_000).following).toBe(false);
      expect(tick(released, T0 + 130_000)).toEqual(FOLLOWING);
    });

    it('最新を追っている間は打っていても追い続ける (「最新へ」で戻ったあと)', () => {
      expect(hold(FOLLOWING)).toBe(FOLLOWING);
      expect(release(FOLLOWING, T0)).toBe(FOLLOWING);
    });

    it('打っている間でも、手で最新まで戻れば追うのを再開する', () => {
      expect(reachLatest()).toEqual(FOLLOWING);
    });

    it('打っていないときに打ち終えても、期限は変わらない', () => {
      const away = leaveLatest(FOLLOWING, T0);
      expect(release(away, T0 + 10_000)).toBe(away);
    });
  });
});
