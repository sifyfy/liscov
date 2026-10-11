import js from '@eslint/js';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';
import betterTailwindcss from 'eslint-plugin-better-tailwindcss';
import { designRules } from './design-lint.config.js';

export default ts.config(
	js.configs.recommended,
	...ts.configs.recommended,
	...svelte.configs['flat/recommended'],
	{
		languageOptions: {
			globals: {
				...globals.browser,
				...globals.node,
			},
		},
	},
	{
		files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
		languageOptions: {
			parserOptions: {
				parser: ts.parser,
			},
		},
		rules: {
			// Svelte 5 runes は let で宣言する必要がある ($state, $derived 等)
			'prefer-const': 'off',
		},
	},
	// デザイントークンの強制 (design-lint.config.js)
	{
		files: ['src/**/*.svelte', 'src/**/*.ts'],
		ignores: ['src/**/*.test.ts', 'src/lib/test/**'],
		plugins: { 'better-tailwindcss': betterTailwindcss },
		settings: { 'better-tailwindcss': { entryPoint: 'src/app.css' } },
		rules: { ...designRules, 'better-tailwindcss/no-unknown-classes': 'error' },
	},
	{
		ignores: [
			'build/',
			'.svelte-kit/',
			'node_modules/',
			'src/lib/types/generated/',
			'.stryker-tmp/',
			'mutants.out*/',
			'reports/',
		],
	},
);
