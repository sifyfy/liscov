/**
 * 流れたものの欄 (スパチャ等の一覧の概要と詳細を含む)の中身 (docs/specs/02_chat.md「配信中の画面」の「流れたもの」「配信の最後のお礼」)
 *
 * 試作 (src/routes/design-proto) も本体の画面もこの関数を使う
 */
import type { GuiChatMessage } from '$lib/types/generated/GuiChatMessage';
import { messagePriority, type MessageOrigin } from './priority';

type Classifiable = Pick<GuiChatMessage, 'message_type' | 'is_first_time_viewer' | 'in_stream_comment_count'>;

/** 自分の配信のスパチャ等 (優先順位 1)。届いた順 (古いものが先) */
export function supportersOf<T extends Classifiable>(messages: readonly T[], originOf: (m: T) => MessageOrigin): readonly T[] {
  return messages.filter((m) => messagePriority(m, originOf(m)) === 1);
}

/** 新規さんの初回コメント (優先順位 2、相手の配信のものを含む)。届いた順 */
export function newcomersOf<T extends Classifiable>(messages: readonly T[], originOf: (m: T) => MessageOrigin): readonly T[] {
  return messages.filter((m) => messagePriority(m, originOf(m)) === 2);
}

/**
 * candidates のうち、流れの画面のいちばん上に見えているもの (firstVisible) より前に届いたもの。新しいものが先。
 * all は届いた順のすべてのメッセージ (candidates と firstVisible はその要素)。
 * 何も見えていなければ (firstVisible が無い) 空にする
 */
export function flowedAbove<T>(candidates: readonly T[], all: readonly T[], firstVisible: T | undefined): readonly T[] {
  if (firstVisible === undefined || candidates.length === 0) return [];
  // 見えているのはたいてい最新の近くなので、後ろから探す
  const boundary = all.lastIndexOf(firstVisible);
  if (boundary <= 0) return [];
  const candidateSet = new Set(candidates);
  return all
    .slice(0, boundary)
    .filter((m) => candidateSet.has(m))
    .reverse();
}

/**
 * 最新から離れている間に届いた優先順位 1・2 の件数 (「最新へ」に添える)。
 * since は離れたときの一覧の最後のもの。一覧 (all、届いた順) に無ければ表示件数で外れたので、一覧のすべてが離れた後に届いたもの
 */
export function arrivedSince<T extends Classifiable>(
  all: readonly T[],
  since: T,
  originOf: (m: T) => MessageOrigin,
): Readonly<{ supporters: number; newcomers: number }> {
  const arrived = all.slice(all.lastIndexOf(since) + 1);
  return { supporters: supportersOf(arrived, originOf).length, newcomers: newcomersOf(arrived, originOf).length };
}
