import type { DarkMode, Theme } from '@tak-ui-lib/components';
import { createMaterialTheme, materialTheme } from '@tak-ui-lib/material';

export interface AppTheme {
  name: string;
  theme: Theme;
}

export const themes = {
  default: {
    name: 'Default',
    theme: materialTheme,
  },
  sky: {
    name: 'Sky',
    theme: createMaterialTheme('#57AEEB'),
  },
  lava: {
    name: 'Lava',
    theme: createMaterialTheme('#FF5733'),
  },
  forest: {
    name: 'Forest',
    theme: createMaterialTheme('#228B22'),
  },
} as const;
export type AppThemeId = keyof typeof themes;
export const appThemeIds = Object.keys(themes) as AppThemeId[];

export const darkModeOptions: DarkMode[] = ['light', 'dark', 'system'];
