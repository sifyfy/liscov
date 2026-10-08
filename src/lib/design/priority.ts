/**
 * 読み取る情報の優先順位 (docs/design/brief.md §2) のうち、コメント 1 件ごとに決まるもの (1〜5)
 *
 * 強調の段階 (docs/design/tokens.md) はこの値から決める。試作も本体の画面もこの関数を使う
 */
import type { GuiChatMessage } from '$lib/types/generated/GuiChatMessage';

export type MessagePriority = 1 | 2 | 3 | 4 | 5;

/** コメントが来た配信。primary は主の配信 (最初の接続)、partner はそれ以外 (docs/specs/02_chat.md「主の配信と相手の配信」) */
export type MessageOrigin = 'primary' | 'partner';

/** 優先順位 1 になる種類 (有料のものとメンバーシップ。マイルストーンは membership に含まれる) */
const PAID_TYPES: ReadonlySet<string> = new Set(['superchat', 'supersticker', 'gift', 'membership', 'membership_gift']);

export function messagePriority(
  message: Readonly<Pick<GuiChatMessage, 'message_type' | 'is_first_time_viewer' | 'in_stream_comment_count'>>,
  origin: MessageOrigin = 'primary',
): MessagePriority {
  // 相手の配信に来たスパチャ等はお礼の対象ではないので、通常のコメントと同じ強さにする
  if (PAID_TYPES.has(message.message_type)) return origin === 'primary' ? 1 : 5;
  const count = message.in_stream_comment_count;
  if (count === null) return 5;
  if (message.is_first_time_viewer) return count === 1 ? 2 : 3;
  return count === 1 ? 4 : 5;
}
