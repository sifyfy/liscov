import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

// https://tauri.app/develop/calling-frontend/#browser-api-access
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
	plugins: [sveltekit(), tailwindcss()],

	// Vite options tailored for Tauri development
	clearScreen: false,
	server: {
		port: 5173,
		strictPort: true,
		host: host || false,
		hmr: host
			? {
					protocol: 'ws',
					host,
					port: 5174,
				}
			: undefined,
		watch: {
			// Tell Vite to ignore watching `src-tauri`
			ignored: ['**/src-tauri/**'],
		},
	},
});
