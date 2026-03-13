/**
 * broadcaster_channel_id のハッシュから安定した色を決定する
 * 同じ配信者には常に同じ色が割り当てられる
 */

/** 色覚多様性を考慮したパレット（8色） */
export const CONNECTION_COLORS = [
  '#4285f4', // 青
  '#ea4335', // 赤
  '#34a853', // 緑
  '#fbbc04', // 黄
  '#ff6d01', // オレンジ
  '#46bdc6', // ティール
  '#ab47bc', // 紫
  '#f06292', // ピンク
] as const;

/** 文字列の簡易ハッシュ（djb2） */
function simpleHash(str: string): number {
  let hash = 5381;
  for (let i = 0; i < str.length; i++) {
    hash = ((hash << 5) + hash) + str.charCodeAt(i);
    hash |= 0;
  }
  return Math.abs(hash);
}

/** broadcaster_channel_id から安定した色を取得 */
export function getConnectionColor(broadcasterChannelId: string): string {
  const index = simpleHash(broadcasterChannelId) % CONNECTION_COLORS.length;
  return CONNECTION_COLORS[index];
}
