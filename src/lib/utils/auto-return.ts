/**
 * 自動スクロールと最新への自動復帰 (docs/specs/02_chat.md「配信中の画面」の「自動スクロール」)
 *
 * 状態は「最新を追っている」「離れていて、いつ戻るか」「離れていて、戻らない (視聴者情報パネルで打っている間)」のどれか。
 * 時刻は呼び出し側が渡す (ミリ秒)。状態は変えずに新しい値を返す
 */

/** 最新から離れたまま操作が無いとき、最新に戻るまでの時間 */
export const AUTO_RETURN_MS = 30_000;

/** deadline が null なら戻らない (打っている間) */
export type AutoReturnState =
  | Readonly<{ following: true; deadline: null }>
  | Readonly<{ following: false; deadline: number | null }>;

export const FOLLOWING: AutoReturnState = { following: true, deadline: null };

const isHeld = (state: AutoReturnState) => !state.following && state.deadline === null;

/** 手でスクロールして最新から離れた。打っている間は戻らないまま */
export function leaveLatest(state: AutoReturnState, now: number): AutoReturnState {
  return isHeld(state) ? state : { following: false, deadline: now + AUTO_RETURN_MS };
}

/** 流れの上で操作した (スクロール・クリック・マウスの移動)。離れている間は期限を延ばす */
export function activity(state: AutoReturnState, now: number): AutoReturnState {
  return state.following || isHeld(state) ? state : { following: false, deadline: now + AUTO_RETURN_MS };
}

/** 視聴者情報パネルで打ちはじめた。離れている間は戻らなくする (追っている間はそのまま) */
export function hold(state: AutoReturnState): AutoReturnState {
  return state.following || isHeld(state) ? state : { following: false, deadline: null };
}

/** 打ち終えた (フォーカスが外れ、保存していない変更も無い)。そこから 30 秒で戻る */
export function release(state: AutoReturnState, now: number): AutoReturnState {
  return isHeld(state) ? { following: false, deadline: now + AUTO_RETURN_MS } : state;
}

/** 手で最新までスクロールした・「最新へ」を押した */
export function reachLatest(): AutoReturnState {
  return FOLLOWING;
}

/** 時間が進んだ。期限を過ぎていれば最新に戻る */
export function tick(state: AutoReturnState, now: number): AutoReturnState {
  return state.deadline !== null && now >= state.deadline ? FOLLOWING : state;
}

/** 最新に戻るまでの残り時間。追っている間と、打っていて戻らない間は null */
export function remainingMs(state: AutoReturnState, now: number): number | null {
  return state.deadline === null ? null : Math.max(0, state.deadline - now);
}
