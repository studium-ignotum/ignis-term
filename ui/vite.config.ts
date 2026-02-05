import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  build: { outDir: 'build' },
  server: {
    allowedHosts: true, // Allows the server to respond to requests from any host
  },
});
