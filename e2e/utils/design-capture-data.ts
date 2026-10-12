import type { addMockMessage } from './test-helpers';

/**
 * デザインの撮影 (design-capture.spec.ts) で流すテストデータ
 *
 * 名前・本文はすべて架空。実在の視聴者の名前やコメントを使わない (スクリーンショットに残るため)。
 * 状態は配信の進み方に合わせて積み上がる順に並べる (通常 → 長い名前・本文 → スパチャ連続 → 大量)。
 */

/** モックサーバーに積むメッセージ (addMockMessage の引数と同じ形) */
export type MockMessage = Readonly<Parameters<typeof addMockMessage>[0]>;

const authors = [
  { author: 'みかん箱', channel_id: 'UC_dc_mikanbako' },
  { author: 'ねむいペンギン', channel_id: 'UC_dc_nemuipenguin', is_member: true },
  { author: 'Kaito_92', channel_id: 'UC_dc_kaito92' },
  { author: 'しおこんぶ', channel_id: 'UC_dc_shiokonbu', is_moderator: true },
  { author: 'yuzu_tea', channel_id: 'UC_dc_yuzutea', is_member: true },
  { author: '夜ふかし研究会', channel_id: 'UC_dc_yofukashi' },
  { author: 'もちもち大福', channel_id: 'UC_dc_mochidaifuku' },
  { author: 'Rin', channel_id: 'UC_dc_rin' },
] as const;

const normalTexts = [
  'こんばんは！',
  'きた',
  '待ってました',
  'このボス強すぎない？',
  'わこつです',
  'ナイス！',
  '今日は何時まで？',
  'さっきのところ右の道だったかも',
  'www',
  '声かわいい',
  'BGM いいね',
  'がんばれ〜',
  'えっ',
  'セーブした？',
  'おつかれさま',
  '初見です！よろしくお願いします',
  '草',
  'それ取り忘れてる',
  'あと少し！',
  '888888',
];

/** 通常のコメント (配信の序盤) */
export const normalMessages: readonly MockMessage[] = normalTexts.map((content, i) => ({
  message_type: 'text',
  ...authors[i % authors.length],
  content,
}));

/** 長い名前・長い本文 (崩れないかを見る) */
export const longMessages: readonly MockMessage[] = [
  {
    message_type: 'text',
    author: 'とても長い名前のチャンネルでございますよろしくお願いいたします公式',
    channel_id: 'UC_dc_longname',
    content: '短い',
  },
  {
    message_type: 'text',
    ...authors[2],
    content:
      'さっきの謎解き、本棚の順番は左から赤・青・緑で、そのあと時計の針を三時に合わせると隠し扉が開くはずです。前作と同じ仕掛けだと思うのでたぶん合ってます！',
  },
  {
    message_type: 'text',
    author: 'VeryLongEnglishChannelNameWithoutAnySpacesForTesting',
    channel_id: 'UC_dc_longascii',
    content: 'https://example.com/a/very/long/url/that/should/wrap/somewhere/without/breaking/the/layout/0123456789',
  },
];

/** スパチャ・メンバーシップの連続 (目立つべきもの)。金額とティアの色は docs/specs/07_revenue.md の表のとおりに組む */
export const superMessages: readonly MockMessage[] = [
  { message_type: 'superchat', ...authors[0], content: 'いつも楽しい配信ありがとう！', amount: '¥500', tier: 'green' },
  { message_type: 'superchat', ...authors[5], content: 'クリアおめでとう！', amount: '¥1,000', tier: 'yellow' },
  { message_type: 'supersticker', ...authors[6], content: '', amount: '¥200', tier: 'cyan' },
  // レベル名は札に出る (02_chat.md)。長いレベル名でも最小幅で名前が潰れないかを見る
  { message_type: 'membership', ...authors[7], content: '', membership_level: '名誉ほしぞらサポーター隊' },
  { message_type: 'superchat', ...authors[1], content: '次の配信も楽しみにしてます', amount: '¥10,000', tier: 'red' },
  { message_type: 'membership_milestone', ...authors[4], content: '1 年経ちました！', milestone_months: 12, membership_level: 'ほしぞら組' },
  { message_type: 'membership_gift', ...authors[3], content: '', gift_count: 5 },
];

/** 大量 (流れの速い配信の終盤) */
export const heavyMessages: readonly MockMessage[] = Array.from({ length: 200 }, (_, i) => ({
  message_type: 'text',
  ...authors[(i * 3) % authors.length],
  content: normalTexts[(i * 7) % normalTexts.length],
}));
