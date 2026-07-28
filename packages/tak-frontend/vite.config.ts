import { fileURLToPath, URL } from 'node:url';

import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import vueDevTools from 'vite-plugin-vue-devtools';
import VueRouter from 'vue-router/vite';
import tailwindcss from '@tailwindcss/vite';
import { templateCompilerOptions } from '@tresjs/core';

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue({ ...templateCompilerOptions }), vueDevTools(), tailwindcss(), VueRouter({})],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
    proxy: {
      '/auth/': {
        target: 'https://localhost',
        secure: false,
      },
      '/api/': {
        target: 'https://localhost',
        secure: false,
      },
    },
  },
});
