/**
 * デザイントークン (docs/design/tokens.md) を強制する lint の設定
 *
 * src のすべての画面に効く。任意値・Tailwind の既定の値・生の色値をエラーにし、
 * Tailwind に登録していないクラスもエラーにする (app.css で Tailwind の既定のテーマを消しているので、
 * 既定の段階に頼ったクラスはここで見つかる)
 */

const DOC = 'docs/design/tokens.md';

/** Tailwind の任意値と、既定のテーマの値 (トークンの代わりに使われやすいもの) */
const RESTRICTED_CLASSES = [
	{
		pattern: '\\[([^\\[\\]]*?)\\](?!:)',
		message: `任意値 "$0" は使わない。トークンを使う (${DOC})`,
	},
	{
		pattern: '^(.*:)?text-(xs|sm|base|lg|[2-9]?xl)$',
		message: `既定の文字サイズ "$0" は使わない。text-caption / label / body / title / display / chat-* を使う (${DOC})`,
	},
	{
		pattern: '^(.*:)?rounded(-[a-z]{1,2})?(-(xs|sm|md|lg|[2-4]?xl))?$',
		message: `既定の角丸 "$0" は使わない。rounded-control / rounded-popup / rounded-full を使う (${DOC})`,
	},
	{
		pattern: '^(.*:)?shadow(-(2xs|xs|sm|md|lg|[2-9]?xl|inner))?$',
		message: `既定の影 "$0" は使わない。shadow-elevation-1 / 2 を使う (${DOC})`,
	},
	{
		pattern:
			'^(.*:)?[a-z-]+-(slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-\\d+(/\\d+)?$',
		message: `既定の色 "$0" は使わない。トークンの色を使う (${DOC})`,
	},
	{
		pattern: '^(.*:)?[a-z-]+-(white|black)(/\\d+)?$',
		message: `既定の色 "$0" は使わない。トークンの色を使う (${DOC})`,
	},
	{
		pattern: '^font-(sans|serif|mono|thin|extralight|light|medium|semibold|extrabold|black)$',
		message: `"$0" は使わない。書体は font-ui / font-code、太さは font-normal / font-bold だけ (${DOC})`,
	},
	{
		pattern: '^(.*:)?(leading|tracking)-',
		message: `"$0" は使わない。行間は文字の段階のトークンに含まれる (${DOC})`,
	},
];

/** 生の色値 (#xxx・rgb()・hsl()) */
const COLOR_LITERAL = '/#[0-9a-fA-F]{3,8}\\b|rgba?\\(|hsla?\\(/';
const COLOR_MESSAGE = `生の色値は使わない。色はトークン (--sys-*) から使う (${DOC})`;

/** デザインの規則 (ESLint の rules) */
export const designRules = {
	'better-tailwindcss/no-restricted-classes': ['error', { restrict: RESTRICTED_CLASSES }],
	'no-restricted-syntax': [
		'error',
		{ selector: `Literal[value=${COLOR_LITERAL}]`, message: COLOR_MESSAGE },
		{ selector: `TemplateElement[value.raw=${COLOR_LITERAL}]`, message: COLOR_MESSAGE },
		{ selector: `SvelteLiteral[value=${COLOR_LITERAL}]`, message: COLOR_MESSAGE },
		{ selector: `SvelteStyleElement SvelteText[value=${COLOR_LITERAL}]`, message: COLOR_MESSAGE },
	],
};
