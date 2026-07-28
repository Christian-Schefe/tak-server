import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import tailwindcss from '@tailwindcss/vite';
import VueRouter from 'vue-router/vite';

export default defineConfig({
  plugins: [
    VueRouter({
      // options
    }),
    vue(),
    tailwindcss(),
  ],
});
