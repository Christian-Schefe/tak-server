import vue from '@vitejs/plugin-vue';
import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import { libInjectCss } from 'vite-plugin-lib-inject-css';
import { glob } from 'glob';

const componentEntries = Object.fromEntries(
  glob.sync('src/components/**/index.ts', { cwd: __dirname }).map((file) => {
    const name = file.split('/')[2];
    return [name, resolve(__dirname, `src/components/${name}/index.ts`)];
  }),
);

export default defineConfig({
  plugins: [vue(), libInjectCss()],
  build: {
    lib: {
      formats: ['es'],
      entry: {
        index: resolve(__dirname, 'src/index.styled.ts'),
        ...componentEntries,
      },
    },
    emptyOutDir: false,
    rolldownOptions: {
      // make sure to externalize deps that shouldn't be bundled
      // into your library
      external: ['vue'],
      output: {
        // Put chunk files at <output>/chunks
        chunkFileNames: 'chunks/[name].[hash].js',
        // Put chunk styles at <output>/assets
        assetFileNames: 'assets/[name][extname]',
        entryFileNames: '[name].js',
        // Provide global variables to use in the UMD build
        // for externalized deps
        globals: {
          vue: 'Vue',
        },
      },
    },
  },
});
