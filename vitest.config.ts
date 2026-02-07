import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

export default defineConfig({
	plugins: [svelte({ hot: false })],
	resolve: {
		alias: {
			$lib: resolve(__dirname, './src/lib'),
		},
		conditions: ['browser'],
	},
	test: {
		include: ['src/**/*.{test,spec}.{js,ts}'],
		environment: 'jsdom',
		globals: true,
		setupFiles: ['./src/lib/test/setup.ts'],
		// Svelte関連の依存関係をインライン化してパフォーマンス向上
		server: {
			deps: {
				inline: [/svelte/],
			},
		},
	},
});
