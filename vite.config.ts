import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	ssr: {
		// xterm-svelte only exports with 'svelte' condition — tell Vite's SSR resolver about it
		noExternal: ['@battlefieldduck/xterm-svelte'],
		resolve: {
			conditions: ['svelte'],
		},
	},
});
