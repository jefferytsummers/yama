import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		port: 5173,
		strictPort: false,
		proxy: {
			// Proxy API requests to embedded server during dev
			'/api': {
				target: 'http://localhost:8080',
				changeOrigin: true
			}
		}
	}
});
