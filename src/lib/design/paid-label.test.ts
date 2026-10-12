/**
 * スパチャ等の札の文言 (docs/specs/02_chat.md「メッセージ表示仕様」の種類ごと)
 *
 * テストの例は仕様の表から取る:
 * - スパチャ・スーパーステッカー: 金額
 * - メンバー加入: 新規メンバー / マイルストーン: メンバー N か月 / メンバーシップギフト: メンバーシップを N 件ギフト
 * - メンバー加入・マイルストーンはレベル名があれば後ろに「 · {レベル名}」
 * - ギフト (ジュエル): ジュエル数があれば「N ジュエル」、無ければギフト名
 * - メンバーシップギフトの受け取り: ギフトで加入 (受け取った人の行は「受け取った人 N: A、B、C」)
 * - 通常のコメント: 札なし
 */
import { describe, expect, it } from 'vitest';
import type { GuiMessageMetadata } from '$lib/types/generated/GuiMessageMetadata';
import { paidLabelOf, recipientsLabelOf } from './paid-label';

const base = { message_type: 'text', amount: null, metadata: null } as const;

const meta = (overrides: Partial<GuiMessageMetadata>): GuiMessageMetadata => ({
    amount: null,
    milestone_months: null,
    membership_level: null,
    gift_count: null,
    gifter: null,
    gift_name: null,
    gift_image_url: null,
    jewel_count: null,
    badges: [],
    badge_info: [],
    is_moderator: false,
    is_verified: false,
    superchat_colors: null,
  ...overrides,
});

describe('paidLabelOf', () => {
  it('スパチャは金額', () => {
    expect(paidLabelOf({ ...base, message_type: 'superchat', amount: '¥10,000' })).toBe('¥10,000');
  });

  it('スーパーステッカーは金額', () => {
    expect(paidLabelOf({ ...base, message_type: 'supersticker', amount: '¥500' })).toBe('¥500');
  });

  it('メンバー加入は「新規メンバー」', () => {
    expect(paidLabelOf({ ...base, message_type: 'membership', metadata: meta({}) })).toBe('新規メンバー');
  });

  it('マイルストーンは「メンバー N か月」', () => {
    expect(paidLabelOf({ ...base, message_type: 'membership', metadata: meta({ milestone_months: 12 }) })).toBe('メンバー 12 か月');
  });

  it('レベル名のあるメンバー加入は「新規メンバー · {レベル名}」', () => {
    expect(paidLabelOf({ ...base, message_type: 'membership', metadata: meta({ membership_level: 'ほしぞら組' }) })).toBe(
      '新規メンバー · ほしぞら組',
    );
  });

  it('レベル名のあるマイルストーンは「メンバー N か月 · {レベル名}」', () => {
    expect(
      paidLabelOf({ ...base, message_type: 'membership', metadata: meta({ milestone_months: 17, membership_level: 'ほしぞら組' }) }),
    ).toBe('メンバー 17 か月 · ほしぞら組');
  });

  it('メンバーシップギフトは「メンバーシップを N 件ギフト」', () => {
    expect(paidLabelOf({ ...base, message_type: 'membership_gift', metadata: meta({ gift_count: 5 }) })).toBe(
      'メンバーシップを 5 件ギフト',
    );
  });

  it('件数が分からないメンバーシップギフトは「メンバーシップのギフト」(0 件と出さない)', () => {
    expect(paidLabelOf({ ...base, message_type: 'membership_gift', metadata: null })).toBe('メンバーシップのギフト');
  });

  it('ジュエル数のあるギフトは「N ジュエル」', () => {
    expect(paidLabelOf({ ...base, message_type: 'gift', metadata: meta({ gift_name: 'Press F', jewel_count: 10 }) })).toBe(
      '10 ジュエル',
    );
  });

  it('ジュエル数の無いギフトはギフト名', () => {
    expect(paidLabelOf({ ...base, message_type: 'gift', metadata: meta({ gift_name: 'Hiding' }) })).toBe('Hiding');
  });

  it('メンバーシップギフトの受け取りは「ギフトで加入」', () => {
    expect(
      paidLabelOf({ ...base, message_type: 'membership_gift_redemption', metadata: meta({ gifter: '@gifter-x9' }) }),
    ).toBe('ギフトで加入');
  });

  it('通常のコメントは札なし', () => {
    expect(paidLabelOf(base)).toBeNull();
  });
});

describe('recipientsLabelOf', () => {
  const recipient = (author: string) => ({ author });

  it('受け取った人を届いた順に「受け取った人 N: A、B、C」', () => {
    expect(recipientsLabelOf([recipient('@a'), recipient('@b'), recipient('@c')])).toBe('受け取った人 3: @a、@b、@c');
  });

  it('1 人でも人数を添える', () => {
    expect(recipientsLabelOf([recipient('@a')])).toBe('受け取った人 1: @a');
  });
});
