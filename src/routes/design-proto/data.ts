/**
 * 方向性の試作 (/design-proto/a|b|c) で共通に使う架空のデータと、場面の切替
 *
 * 3 案を同じデータで撮り比べるためのもの。名前・本文は架空。
 * 配信の途中 (21:30 ごろ) の 1 場面で、次を含む:
 * - 自分の配信のスパチャ等 6 件。うち赤スパ (¥10,000) は 40 件以上前に流れていて、普通の一覧では画面の外
 * - 新規さんの初回と 2 回目以降、常連の配信内初回、長い本文
 * - コラボの場面では、相手の配信 (あおば) のコメント・新規さん・スパチャ・参加希望が混ざる
 *
 * 場面は URL の ?scene= で切り替える (normal / collab / thanks / party / heavy)。ブリーフ §1「場面」。
 * heavy は、スパチャ等と新規さんが大量に流れた後 (破綻しないことの確認用)
 */
import type { GuiChatMessage } from '$lib/types/generated/GuiChatMessage';
import type { SuperChatColors } from '$lib/types/generated/SuperChatColors';
import { messagePriority, type MessageOrigin, type MessagePriority } from '$lib/design/priority';
import { newcomersOf as newcomersOfFeed, supportersOf as supportersOfFeed } from '$lib/design/flow';

type Message = Omit<GuiChatMessage, 'id'>;

/**
 * YouTube から届くスパチャのティアの色 (docs/design/tokens.md「強調」: トークンにせずデータとして扱う)。
 * 本体では metadata.superchat_colors で届く値なので、試作でもデータとして生の色値を書く。
 * 見出しの色と金額の組み合わせは docs/specs/07_revenue.md の表のとおり
 * (青 ¥100・水色 ¥200・緑 ¥500・黄 ¥1,000・オレンジ ¥2,000・マゼンタ ¥5,000・赤 ¥10,000)。本文の背景色は仕様に無く、YouTube の実際の値に近いもの
 */
/* eslint-disable no-restricted-syntax -- YouTube から届くデータの色の再現 (上の注釈) */
const TIER = {
  blue: { header_background: '#1565c0', header_text: '#ffffff', body_background: '#1565c0', body_text: '#ffffff' },
  cyan: { header_background: '#00b8d4', header_text: '#000000', body_background: '#00e5ff', body_text: '#000000' },
  green: { header_background: '#00bfa5', header_text: '#000000', body_background: '#1de9b6', body_text: '#000000' },
  yellow: { header_background: '#ffb300', header_text: '#000000', body_background: '#ffca28', body_text: '#000000' },
  orange: { header_background: '#e65100', header_text: '#ffffff', body_background: '#f57c00', body_text: '#ffffff' },
  magenta: { header_background: '#c2185b', header_text: '#ffffff', body_background: '#e91e63', body_text: '#ffffff' },
  red: { header_background: '#d00000', header_text: '#ffffff', body_background: '#e62117', body_text: '#ffffff' },
} as const satisfies Record<string, SuperChatColors>;
/* eslint-enable no-restricted-syntax */

function base(author: string, time: string): Omit<Message, 'message_type' | 'content' | 'runs'> {
  return {
    timestamp: time,
    timestamp_usec: '0',
    author,
    author_icon_url: null,
    channel_id: `UC-${author}`,
    amount: null,
    is_member: false,
    is_first_time_viewer: false,
    in_stream_comment_count: null,
    metadata: null,
    connection_id: PRIMARY_CONNECTION,
    platform: 'youtube',
    broadcaster_name: '試作の配信者',
    author_reading: null,
  };
}

const PRIMARY_CONNECTION = 1n;
const PARTNER_CONNECTION = 2n;
const PARTNER_NAME = 'あおば';

/** 相手の配信 (コラボ) に来たものにする */
const fromPartner = (m: Message): Message => ({
  ...m,
  connection_id: PARTNER_CONNECTION,
  broadcaster_name: PARTNER_NAME,
  channel_id: `${m.channel_id}-partner`,
});

const metadata = (overrides: Partial<NonNullable<Message['metadata']>>): NonNullable<Message['metadata']> => ({
  amount: null,
  milestone_months: null,
  gift_count: null,
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

type Viewer = 'regular' | 'new';

/** 通常のコメント。count は配信内のコメント回数 (#N) */
function text(author: string, time: string, content: string, count: number, viewer: Viewer = 'regular'): Message {
  return {
    ...base(author, time),
    message_type: 'text',
    content,
    runs: [{ type: 'Text', content }],
    is_first_time_viewer: viewer === 'new',
    in_stream_comment_count: count,
  };
}

function superchat(author: string, time: string, content: string, amount: string, colors: SuperChatColors | null): Message {
  return {
    ...base(author, time),
    message_type: 'superchat',
    content,
    runs: content ? [{ type: 'Text', content }] : [],
    amount,
    metadata: metadata({ amount, superchat_colors: colors }),
  };
}

function sticker(author: string, time: string, amount: string, colors: SuperChatColors): Message {
  return {
    ...base(author, time),
    message_type: 'supersticker',
    content: '',
    runs: [],
    amount,
    metadata: metadata({ amount, superchat_colors: colors }),
  };
}

/** メンバー加入 (months が null) とマイルストーン (months が数) */
function membership(author: string, time: string, content: string, months: number | null): Message {
  return {
    ...base(author, time),
    message_type: 'membership',
    content,
    runs: content ? [{ type: 'Text', content }] : [],
    is_member: true,
    metadata: metadata({ milestone_months: months }),
  };
}

function membershipGift(author: string, time: string, count: number): Message {
  return {
    ...base(author, time),
    message_type: 'membership_gift',
    content: '',
    runs: [],
    is_member: true,
    metadata: metadata({ gift_count: count }),
  };
}

const SEEDS: readonly Message[] = [
  text('しおこんぶ', '21:12', 'こんばんは〜', 1),
  text('夜ふかし研究会', '21:12', 'きたきた', 1),
  superchat('ねこ山', '21:13', '', '¥200', TIER.cyan),
  text('しおこんぶ', '21:13', '今日はどこまで進む予定？', 2),
  text('Kaito_92', '21:13', 'はじめまして！切り抜きから来ました', 1, 'new'),
  text('たまごやき', '21:14', 'こんばんは', 1),
  sticker('みずいろ', '21:14', '¥500', TIER.green),
  text('夜ふかし研究会', '21:14', '前回のボス戦の続きからだよね', 2),
  text('Kaito_92', '21:15', '雰囲気いいですね', 2, 'new'),
  membership('のりたま', '21:15', '12 か月ありがとう！', 12),
  text('たまごやき', '21:16', 'BGM すき', 2),
  text('しおこんぶ', '21:16', 'そこ左の部屋にセーブポイントあったはず', 3),
  superchat(
    'ねむいペンギン',
    '21:17',
    '遅れてきました！今日も見られてうれしいです。新しい衣装めちゃくちゃ似合ってます、次の配信も楽しみにしてます',
    '¥10,000',
    TIER.red,
  ),
  text('夜ふかし研究会', '21:17', 'うわ赤スパだ', 3),
  text('みずいろ', '21:18', 'おつかれさまです', 1),
  text('しおこんぶ', '21:18', 'ここの謎解き、本棚の順番は左から赤・青・緑だった気がする', 4),
  text('たまごやき', '21:18', 'ｗｗｗ', 3),
  text('Kaito_92', '21:19', 'なるほど', 3, 'new'),
  text('夜ふかし研究会', '21:19', 'うしろ！うしろ！', 4),
  text('みずいろ', '21:19', '今の避け方うますぎ', 2),
  text('しおこんぶ', '21:20', '弾が足りるか心配', 5),
  text('たまごやき', '21:20', 'がんばれー', 4),
  text('ひだまり', '21:20', '初見です、声がきれいですね', 1, 'new'),
  text('夜ふかし研究会', '21:21', 'ハンドガンの弾は右の棚', 5),
  text('みずいろ', '21:21', 'こわいこわい', 3),
  text('しおこんぶ', '21:21', 'ナイス！', 6),
  text('のりたま', '21:22', 'いつもありがとう〜', 1),
  text('たまごやき', '21:22', 'そこ曲がったらゾンビいるよ', 5),
  text('Kaito_92', '21:22', 'チャンネル登録しました', 4, 'new'),
  text('夜ふかし研究会', '21:23', '倒した！', 6),
  membershipGift('ぺんぎん村', '21:23', 5),
  text('みずいろ', '21:23', 'ギフトありがとうございます', 4),
  text('しおこんぶ', '21:24', 'メンバーになれた！', 7),
  text('ひだまり', '21:24', 'この先のエリアも楽しみ', 2, 'new'),
  text('たまごやき', '21:24', '鍵を取り忘れてない？', 6),
  text('夜ふかし研究会', '21:25', 'さっきの部屋の机の上にあったよ', 7),
  text('のりたま', '21:25', 'セーブしておこう', 2),
  text('みずいろ', '21:25', 'ここ BGM 変わるのいいね', 5),
  text('しおこんぶ', '21:26', 'いったん回復しよう', 8),
  text('たまごやき', '21:26', '体力やばい', 7),
  text('Kaito_92', '21:26', 'ドキドキする', 5, 'new'),
  text('夜ふかし研究会', '21:27', 'ここのボス、最初は足を狙うと楽', 8),
  text('ゆずぽん', '21:27', 'こんばんはー！', 1),
  text('みずいろ', '21:27', '足ねらい有効だった', 6),
  text('しおこんぶ', '21:28', 'いけるいける', 9),
  text('のりたま', '21:28', '今日のサムネかわいい', 3),
  text('たまごやき', '21:28', '惜しい！', 8),
  text('ゆずぽん', '21:28', '遅刻しました、どこまで進みました？', 2),
  text('夜ふかし研究会', '21:29', 'ボス戦 2 回目', 9),
  text('ひだまり', '21:29', '応援してます', 3, 'new'),
  text('みずいろ', '21:29', 'あと少し', 7),
  text('しおこんぶ', '21:30', '第 2 形態きた', 10),
  text('たまごやき', '21:30', 'うおおお', 9),
  text('こもれび', '21:30', 'はじめてコメントします！ずっと見てました', 1, 'new'),
  text('夜ふかし研究会', '21:30', '弾切れ注意', 10),
  text('ゆずぽん', '21:31', 'がんばって〜', 3),
  superchat('みかん箱', '21:31', 'ボス戦がんばって！これで回復薬買ってください', '¥2,000', TIER.orange),
  text('みずいろ', '21:31', 'いけ！', 8),
  text('しおこんぶ', '21:31', '倒したああああ', 11),
  text('のりたま', '21:32', 'おめでとう！', 4),
  text('たまごやき', '21:32', 'ナイスすぎる', 10),
  text('ひだまり', '21:32', 'すごい！', 4, 'new'),
];

/** 相手の配信に来たもの (コラボ・参加型の場面だけ) */
const PARTNER_SEEDS: readonly Message[] = [
  text('そら豆', '21:14', 'あおばちゃんのところから来ました', 1),
  text('ねぎま', '21:20', '参加希望です！', 1),
  text('Riku', '21:24', '参加希望！ランクはゴールドです', 1, 'new'),
  superchat('まつぼっくり', '21:26', 'あおばちゃんいつも応援してます', '¥1,000', TIER.yellow),
  text('ねぎま', '21:28', 'よろしくお願いします！', 2),
  text('そら豆', '21:29', 'コラボ最高', 2),
  text('くろまめ', '21:30', '次の試合に参加したいです', 1),
  text('Riku', '21:31', 'がんばります', 2, 'new'),
  text('まつぼっくり', '21:32', 'ナイス〜', 2),
].map(fromPartner);

/** 大量の場面で、流れの前のほうに足すもの (スパチャ等 15 件、新規さん 8 人) */
const HEAVY_TIERS = [TIER.cyan, TIER.green, TIER.yellow, TIER.magenta] as const;
const HEAVY_AMOUNTS = ['¥200', '¥500', '¥1,000', '¥5,000'] as const;
const HEAVY_SEEDS: readonly Message[] = [
  ...['あんず', 'いぶき', 'うみねこ', 'えだまめ', 'おもち', 'かすてら', 'きなこ', 'くるみ', 'けやき', 'こはく', 'さくらもち', 'しらたま', 'すだち', 'せんべい', 'そばちゃ'].map(
    (name, i) =>
      superchat(
        name,
        `21:${String(i).padStart(2, '0')}`,
        i % 4 === 3
          ? '長いメッセージの例です。今日の配信もとても楽しかったです。前回の続きがずっと気になっていたので、見られてうれしいです。体に気をつけてがんばってください'
          : '',
        HEAVY_AMOUNTS[i % 4],
        HEAVY_TIERS[i % 4],
      ),
  ),
  ...['なのはな', 'にじます', 'ぬいぐるみ', 'ねりあめ', 'のはら', 'はなび', 'ひなた', 'ふゆび'].map((name, i) =>
    text(name, `21:0${i}`, 'はじめまして！おすすめに出てきて来ました。これからよろしくお願いします', 1, 'new'),
  ),
];

/** ギフト (ジュエル)。画像は無し (読めないときの形) */
function gift(author: string, time: string, giftName: string, jewels: number): Message {
  return {
    ...base(author, time),
    message_type: 'gift',
    content: giftName,
    runs: [],
    in_stream_comment_count: 2,
    metadata: metadata({ gift_name: giftName, jewel_count: jewels }),
  };
}

function system(time: string, content: string): Message {
  return { ...base('', time), message_type: 'system', content, runs: [{ type: 'Text', content }] };
}

/**
 * コメントの行の見本 (/design-tokens と見た目の回帰テスト e2e/chat-row-visual.spec.ts)。
 * 優先順位 1〜5・種類・主の配信と相手の配信・読み仮名・バッジ・長い本文の折り返しを 1 つずつ並べる。
 * 時刻は「21:12」の形の文字列なので、表示はタイムゾーンによらない
 */
const ROW_SEEDS: readonly Message[] = [
  text('ひだまり', '21:20', '初見です、声がきれいですね', 1, 'new'),
  text('Kaito_92', '21:22', 'チャンネル登録しました', 4, 'new'),
  text('ゆずぽん', '21:27', 'こんばんはー！', 1),
  { ...text('しおこんぶ', '21:28', 'いけるいける', 9), author_reading: 'しおこんぶ', metadata: metadata({ is_moderator: true }) },
  { ...text('のりたま', '21:28', '今日のサムネかわいい', 3), is_member: true },
  text(
    'とても長い名前のチャンネルでございますよろしくお願いいたします公式',
    '21:29',
    'ここの謎解き、本棚の順番は左から赤・青・緑だった気がする。違ったら次は緑・赤・青の順で試してみて、それでもだめなら机の引き出しのメモを見直すといいかも',
    5,
  ),
  superchat('ねむいペンギン', '21:17', '遅れてきました！今日も見られてうれしいです', '¥10,000', TIER.red),
  sticker('みずいろ', '21:14', '¥500', TIER.green),
  superchat('みかん箱', '21:31', '色の情報が無いスパチャ', '¥2,000', null),
  membership('のりたま', '21:15', '12 か月ありがとう！', 12),
  membershipGift('ぺんぎん村', '21:23', 5),
  gift('こもれび', '21:24', 'ハート', 100),
  system('21:25', 'モデレーターがメッセージを削除しました'),
  ...[
    text('そら豆', '21:14', 'あおばちゃんのところから来ました', 1),
    text('Riku', '21:24', '参加希望！ランクはゴールドです', 1, 'new'),
    superchat('まつぼっくり', '21:26', 'あおばちゃんいつも応援してます', '¥1,000', TIER.yellow),
  ].map(fromPartner),
];

const withIds = (seeds: readonly Message[], prefix: string): readonly GuiChatMessage[] =>
  seeds.map((m, i) => ({ ...m, id: `${prefix}${i}` }));

/** 時刻順に混ぜる (同じ時刻なら元の順) */
const merged = (a: readonly GuiChatMessage[], b: readonly GuiChatMessage[]): readonly GuiChatMessage[] =>
  [...a, ...b].map((m, i) => ({ m, i })).sort((x, y) => x.m.timestamp.localeCompare(y.m.timestamp) || x.i - y.i).map(({ m }) => m);

const PRIMARY_FEED = withIds(SEEDS, 'm');
const COLLAB_FEED = merged(PRIMARY_FEED, withIds(PARTNER_SEEDS, 'p'));
const HEAVY_FEED = merged(PRIMARY_FEED, withIds(HEAVY_SEEDS, 'h'));

export const ROW_SAMPLES = withIds(ROW_SEEDS, 'r');

/** ブリーフ §1 の場面。party は参加型のコラボ (2 つの配信 + 参加の一覧) */
export type Scene = 'normal' | 'collab' | 'thanks' | 'party' | 'heavy';

const SCENES = ['normal', 'collab', 'thanks', 'party', 'heavy'] as const satisfies readonly Scene[];

export function sceneOf(url: URL): Scene {
  const scene = url.searchParams.get('scene');
  return SCENES.find((s) => s === scene) ?? 'normal';
}

/** 場面ごとのコメントの流れ */
export function feedOf(scene: Scene): readonly GuiChatMessage[] {
  if (scene === 'collab' || scene === 'party') return COLLAB_FEED;
  if (scene === 'heavy') return HEAVY_FEED;
  return PRIMARY_FEED;
}

export { paidLabelOf } from '$lib/design/paid-label';

export const originOf = (m: GuiChatMessage): MessageOrigin => (m.connection_id === PARTNER_CONNECTION ? 'partner' : 'primary');

export const priorityOf = (m: GuiChatMessage): MessagePriority => messagePriority(m, originOf(m));

/** 配信の最後のお礼の対象 (自分の配信のスパチャ等、時刻順)。ブリーフ §2 */
export const supportersOf = (feed: readonly GuiChatMessage[]): readonly GuiChatMessage[] => supportersOfFeed(feed, originOf);

/** 新規さんの初回コメント (新しい順) */
export const newcomersOf = (feed: readonly GuiChatMessage[]): readonly GuiChatMessage[] => [...newcomersOfFeed(feed, originOf)].reverse();

type Player = Readonly<{ name: string; origin: MessageOrigin }>;

/** 参加型の一覧 (参加の受け付け方は未決なので、試作では固定のデータ) */
export const party = {
  playing: [
    { name: 'しおこんぶ', origin: 'primary' },
    { name: 'ねぎま', origin: 'partner' },
  ],
  waiting: [
    { name: 'Riku', origin: 'partner' },
    { name: 'ゆずぽん', origin: 'primary' },
    { name: 'くろまめ', origin: 'partner' },
  ],
} as const satisfies Record<string, readonly Player[]>;

export const partnerName = PARTNER_NAME;

export const formatCount = (n: number): string => n.toLocaleString('ja-JP');

/** 一覧を最新 (いちばん下) に合わせ続ける ({@attach})。ウィンドウの大きさが変わっても最新が見える */
export function stickToBottom(node: HTMLElement): () => void {
  const observer = new ResizeObserver(() => {
    node.scrollTop = node.scrollHeight;
  });
  observer.observe(node);
  return () => observer.disconnect();
}

/** 優先順位 6〜8 (同接・いいね・チャット率、リアクションの勢い、総数) */
export const stats = {
  viewers: 1234,
  likes: 567,
  chatPerMinute: 48,
  comments: 1820,
  commenters: 312,
} as const;

export const reactions = {
  rising: true,
  counts: [
    { emoji: '❤️', count: 128 },
    { emoji: '😂', count: 64 },
    { emoji: '🎉', count: 31 },
  ],
} as const;
