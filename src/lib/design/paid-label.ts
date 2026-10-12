/**
 * スパチャ等の札の文言 (docs/specs/02_chat.md「メッセージ表示仕様」)。
 * 流れの面の見出し、相手の配信のスパチャ等に添える札、流れたものの欄 (スパチャ等の一覧の詳細を含む) で使う
 */
import type { GuiChatMessage } from '$lib/types/generated/GuiChatMessage';

export function paidLabelOf(
  message: Readonly<Pick<GuiChatMessage, 'message_type' | 'amount' | 'metadata'>>,
): string | null {
  const meta = message.metadata;
  switch (message.message_type) {
    case 'superchat':
    case 'supersticker':
      return message.amount;
    case 'membership':
      return meta?.milestone_months ? `メンバー ${meta.milestone_months} か月` : '新規メンバー';
    case 'membership_gift':
      return meta?.gift_count != null ? `メンバーシップを ${meta.gift_count} 件ギフト` : 'メンバーシップのギフト';
    case 'membership_gift_redemption':
      return 'ギフトで加入';
    case 'gift':
      return meta?.jewel_count != null ? `${meta.jewel_count} ジュエル` : (meta?.gift_name ?? 'ギフト');
    default:
      return null;
  }
}

/** メンバーシップギフトの受け取った人の行の文言 (docs/specs/02_chat.md「受け取った人の行」)。届いた順 */
export function recipientsLabelOf(recipients: readonly Readonly<Pick<GuiChatMessage, 'author'>>[]): string {
  return `受け取った人 ${recipients.length}: ${recipients.map((r) => r.author).join('、')}`;
}
