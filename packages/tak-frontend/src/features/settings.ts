import {
  darkModeOptions,
  primeVueThemeIds,
  themes,
  type DarkModeOption,
  type PrimeVueThemeId,
} from '@/features/appTheme';
import { board2dThemeIds } from '@/features/board2dThemes';
import { board3dPiecePresets, board3dTilesPresets } from '@/features/board3dResources';
import { ninja2dThemes } from '@/features/ninjaThemes';
import { usePreset } from '@primeuix/themes';
import { useMediaQuery, useStorage } from '@vueuse/core';
import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import { z } from 'zod';

function enumOrDefault<T extends string>(values: readonly T[], defaultValue: T) {
  return z.enum(values).default(defaultValue);
}

const settingsSchema = z.object({
  theme: enumOrDefault(primeVueThemeIds, 'default'),
  darkMode: enumOrDefault(darkModeOptions, 'system'),
  boardType: enumOrDefault(['ninja', '2d', '3d'], '2d'),
  boardTypeSettings: z.object({
    ninja: z.object({
      colorTheme: enumOrDefault(ninja2dThemes, 'discord'),
      axisLabels: enumOrDefault(['normal', 'small', 'none'], 'normal'),
      highlightSquares: z.boolean(),
      animateBoard: z.boolean(),
      board3d: z.boolean(),
      orthographic: z.boolean(),
      perspective: z.number(),
    }),
    '2d': z.object({
      theme: enumOrDefault(board2dThemeIds, 'classic'),
      axisLabelSize: z.number(),
      axisLabels: z.boolean(),
    }),
    '3d': z.object({
      piecePreset: enumOrDefault(
        board3dPiecePresets.map((preset) => preset.id),
        'basic',
      ),
      tilesPreset: enumOrDefault(
        board3dTilesPresets.map((preset) => preset.id),
        'basic',
      ),
      pieceScale: z.number(),
    }),
  }),
  audioVolume: z.number().min(0).max(1).default(0.5),
});
export type Settings = z.infer<typeof settingsSchema>;

function tryParse<T>(json: string | null, schema: z.ZodType<T>, defaultValue: T): T {
  if (json !== null) {
    try {
      const parsed = JSON.parse(json);
      return schema.parse(parsed);
    } catch (e) {
      console.error('Failed to load settings from localStorage, using defaults.', e);
    }
  }
  return defaultValue;
}

export const useSettingsStore = defineStore('settings', () => {
  const storedSettings = useStorage<string | null>('settings-general', null);

  const settings = ref<Settings>(
    tryParse(storedSettings.value, settingsSchema, {
      theme: 'default',
      darkMode: 'system',
      boardType: '2d',
      boardTypeSettings: {
        ninja: {
          colorTheme: 'discord',
          axisLabels: 'normal',
          highlightSquares: true,
          animateBoard: true,
          board3d: false,
          orthographic: false,
          perspective: 45,
        },
        '2d': {
          theme: 'classic',
          axisLabelSize: 12,
          axisLabels: true,
        },
        '3d': {
          piecePreset: 'basic',
          tilesPreset: 'basic',
          pieceScale: 0.9,
        },
      },
      audioVolume: 0.5,
    }),
  );

  watch(
    settings,
    (newSettings) => {
      storedSettings.value = JSON.stringify(newSettings);
    },
    { immediate: true, deep: true },
  );

  const prefersDark = useMediaQuery('(prefers-color-scheme: dark)');
  watch(prefersDark, () => {
    if (settings.value.darkMode === 'system') {
      applyDarkMode('system');
    }
  });

  function applyTheme(themeId: PrimeVueThemeId) {
    const theme = themes[themeId];
    usePreset(theme.primengTheme);
  }

  function applyDarkMode(mode: DarkModeOption) {
    const isDark = mode === 'dark' || (mode === 'system' && prefersDark.value);
    document.documentElement.classList.toggle('dark-mode', isDark);
  }

  function initializeSettings() {
    applyTheme(settings.value.theme);
    applyDarkMode(settings.value.darkMode);
  }

  function setTheme(themeId: PrimeVueThemeId) {
    settings.value.theme = themeId;
    applyTheme(themeId);
  }

  function setDarkMode(mode: DarkModeOption) {
    settings.value.darkMode = mode;
    applyDarkMode(mode);
  }

  return {
    settings,
    setTheme,
    setDarkMode,
    initializeSettings,
  };
});
