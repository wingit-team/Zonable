import { defineConfig } from 'vite';
import { fileURLToPath, URL } from 'node:url';
import wasm from 'vite-plugin-wasm';
import solidPlugin from 'vite-plugin-solid';

export default defineConfig({
  plugins: [solidPlugin(), wasm()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  build: {
    target: 'esnext',
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('node_modules/@babylonjs')) {
            return 'vendor';
          }
          if (id.includes('node_modules/solid-js')) {
            return 'ui';
          }
          if (id.includes('.worker.')) {
            return 'workers';
          }
          return undefined;
        }
      }
    }
  },
  server: {
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp'
    }
  },
  worker: {
    format: 'es'
  }
});
