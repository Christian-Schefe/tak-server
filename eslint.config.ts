import { globalIgnores } from 'eslint/config';
import { defineConfigWithVueTs, vueTsConfigs } from '@vue/eslint-config-typescript';
import pluginVue from 'eslint-plugin-vue';
import skipFormatting from 'eslint-config-prettier/flat';
import tseslint from 'typescript-eslint';
import js from '@eslint/js';

export default defineConfigWithVueTs(
  pluginVue.configs['flat/recommended'],
  {
    name: 'config-app',
    files: ['**/*.{vue,ts,mts,tsx}'],
    languageOptions: {
      parserOptions: {
        projectServices: [
          './tsconfig.json',
          './packages/tak-frontend/tsconfig.json',
          './packages/tak-ui/*/tsconfig.json',
        ],
      },
    },
    extends: [js.configs.recommended, tseslint.configs.strictTypeChecked, vueTsConfigs.recommended],
    rules: {
      eqeqeq: 'error',
      'prefer-template': 'error',
      quotes: ['error', 'single'],
      '@typescript-eslint/no-unsafe-assignment': 'warn',
      '@typescript-eslint/no-unsafe-call': 'warn',
      '@typescript-eslint/no-unsafe-member-access': 'warn',
      '@typescript-eslint/strict-boolean-expressions': 'error',
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-unsafe-function-type': 'error',
      '@typescript-eslint/no-unsafe-return': 'error',
      'vue/multi-word-component-names': 'off',
      '@typescript-eslint/restrict-template-expressions': 'off',
    },
  },
  globalIgnores([
    '**/dist/**',
    '**/dist-ssr/**',
    '**/coverage/**',
    '**/node_modules/**',
    'src/tak-wasm-engine/**',
  ]),
  skipFormatting,
);
