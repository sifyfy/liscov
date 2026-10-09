/**
 * 自動スクロールと最新への自動復帰 (docs/specs/02_chat.md「配信中の画面」の「自動スクロール」)
 *
 * 状態は「最新を追っている」か「離れていて、いつ戻るか」のどちらか。時刻は呼び出し側が渡す (ミリ秒)。
 * 状態は変えずに新しい値を返す
 */

/** 最新から離れたまま操作が無いとき、最新に戻るまでの時間 */
export const AUTO_RETURN_MS = 30_000;

export type AutoReturnState =
  | Readonly<{ following: true; deadline: null }>
  | Readonly<{ following: false; deadline: number }>;

export const FOLLOWING: AutoReturnState = { following: true, deadline: null };

/** 手でスクロールして最新から離れた */
export function leaveLatest(_state: AutoReturnState, now: number): AutoReturnState {
  return { following: false, deadline: now + AUTO_RETURN_MS };
}

/** 流れの上で操作した (スクロール・クリック・マウスの移動)。離れている間は期限を延ばす */
export function activity(state: AutoReturnState, now: number): AutoReturnState {
  return state.following ? state : { following: false, deadline: now + AUTO_RETURN_MS };
}

/** 手で最新までスクロールした・「最新へ」を押した */
export function reachLatest(): AutoReturnState {
  return FOLLOWING;
}

/** 時間が進んだ。期限を過ぎていれば最新に戻る */
export function tick(state: AutoReturnState, now: number): AutoReturnState {
  return !state.following && now >= state.deadline ? FOLLOWING : state;
}

/** 最新に戻るまでの残り時間。追っている間は null */
export function remainingMs(state: AutoReturnState, now: number): number | null {
  return state.following ? null : Math.max(0, state.deadline - now);
}
