// 表示用フォーマット関数（複数コンポーネントで共有）

// Intl.NumberFormat の生成は高コストなのでモジュールレベルで一度だけ行う
const JPY_FORMATTER = new Intl.NumberFormat('ja-JP', {
  style: 'currency',
  currency: 'JPY',
  maximumFractionDigits: 0,
});

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
