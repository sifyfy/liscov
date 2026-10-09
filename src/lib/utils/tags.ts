/**
 * 視聴者のタグの入力欄の文字とタグの並び (docs/specs/06_viewer.md「視聴者のタブ」)
 */

/** 読点 (、) かカンマ (,) で区切る。前後の空白と空のタグは除く */
export function parseTags(text: string): string[] {
  return text
    .split(/[、,]/)
    .map((tag) => tag.trim())
    .filter((tag) => tag !== '');
}

/** 入力欄に出す形 (読点でつなぐ) */
export function formatTags(tags: readonly string[]): string {
  return tags.join('、');
}
