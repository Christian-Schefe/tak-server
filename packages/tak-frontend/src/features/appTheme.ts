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
  castle: {
    name: 'Sky',
    theme: createMaterialTheme('#2233FF'),
  },
} as const;
export type AppThemeId = keyof typeof themes;
export const appThemeIds = Object.keys(themes) as AppThemeId[];

export const darkModeOptions: DarkMode[] = ['light', 'dark', 'system'];
