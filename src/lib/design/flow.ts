/**
 * 流れたものの欄とお礼の一覧の中身 (docs/specs/02_chat.md「配信中の画面」の「流れたもの」「配信の最後のお礼」)
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
