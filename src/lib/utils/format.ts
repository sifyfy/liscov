// 表示用フォーマット関数（複数コンポーネントで共有）

// Intl.NumberFormat の生成は高コストなのでモジュールレベルで一度だけ行う
const JPY_FORMATTER = new Intl.NumberFormat('ja-JP', {
  style: 'currency',
  currency: 'JPY',
  maximumFractionDigits: 0,
});
const COUNT_FORMATTER = new Intl.NumberFormat('ja-JP');

/** 件数・人数を 3 桁ごとに区切る（例: 1,820） */
export function formatCount(n: number): string {
  return COUNT_FORMATTER.format(n);
}

/**
 * 視聴者の貢献額を日本円通貨表記にする。0 は「貢献なし」として '-' を返す
 */
export function formatContribution(amount: number): string {
  if (amount === 0) return '-';
  return JPY_FORMATTER.format(amount);
}

/**
 * タイムスタンプをローカルタイムゾーンの HH:MM:SS（24時間・2桁ゼロ埋め）にする。
 * 空文字列は空のまま、解釈できない文字列は原文をそのまま返す。
 * チャット欄ではメッセージごとに呼ばれるため、toLocaleTimeString ではなく手動整形で軽量に保つ
 */
export function formatTimestamp(timestamp: string): string {
  if (!timestamp) return '';
  const date = new Date(timestamp);
  if (isNaN(date.getTime())) {
    return timestamp;
  }
  const h = String(date.getHours()).padStart(2, '0');
  const m = String(date.getMinutes()).padStart(2, '0');
  const s = String(date.getSeconds()).padStart(2, '0');
  return `${h}:${m}:${s}`;
}

/**
 * 日付をローカルタイムゾーンの短い形にする。今年なら「10/9」、それより前は「2025/10/1」。
 * 解釈できない文字列は原文をそのまま返す (06_viewer.md「視聴者のタブ」の最終確認)
 */
export function formatDay(timestamp: string, now: Date = new Date()): string {
  const date = new Date(timestamp);
  if (isNaN(date.getTime())) return timestamp;
  const day = `${date.getMonth() + 1}/${date.getDate()}`;
  return date.getFullYear() === now.getFullYear() ? day : `${date.getFullYear()}/${day}`;
}
