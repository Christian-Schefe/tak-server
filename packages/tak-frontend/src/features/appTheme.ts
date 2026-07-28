import { definePreset } from '@primeuix/themes';
import Aura from '@primeuix/themes/aura';
import type { Preset } from '@primeuix/themes/types';

function themeColors(primary: string | object, surface: string | object) {
  return {
    semantic: {
      primary:
        typeof primary === 'string'
          ? {
              50: `{${primary}.50}`,
              100: `{${primary}.100}`,
              200: `{${primary}.200}`,
              300: `{${primary}.300}`,
              400: `{${primary}.400}`,
              500: `{${primary}.500}`,
              600: `{${primary}.600}`,
              700: `{${primary}.700}`,
              800: `{${primary}.800}`,
              900: `{${primary}.900}`,
              950: `{${primary}.950}`,
            }
          : primary,
      colorScheme: {
        light: {
          surface:
            typeof surface === 'string'
              ? {
                  0: '#ffffff',
                  50: `{${surface}.50}`,
                  100: `{${surface}.100}`,
                  200: `{${surface}.200}`,
                  300: `{${surface}.300}`,
                  400: `{${surface}.400}`,
                  500: `{${surface}.500}`,
                  600: `{${surface}.600}`,
                  700: `{${surface}.700}`,
                  800: `{${surface}.800}`,
                  900: `{${surface}.900}`,
                  950: `{${surface}.950}`,
                }
              : surface,
        },
        dark: {
          surface:
            typeof surface === 'string'
              ? {
                  0: '#ffffff',
                  50: `{${surface}.50}`,
                  100: `{${surface}.100}`,
                  200: `{${surface}.200}`,
                  300: `{${surface}.300}`,
                  400: `{${surface}.400}`,
                  500: `{${surface}.500}`,
                  600: `{${surface}.600}`,
                  700: `{${surface}.700}`,
                  800: `{${surface}.800}`,
                  900: `{${surface}.900}`,
                  950: `{${surface}.950}`,
                }
              : surface,
        },
      },
    },
  };
}
const shark = {
  0: '#f5f5f7',
  50: '#dfdfe2',
  100: '#cacace',
  200: '#b5b5ba',
  300: '#a0a1a6',
  400: '#8c8d93',
  500: '#787980',
  600: '#64676e',
  700: '#52545c',
  800: '#40434b',
  900: '#2e323a',
  950: '#1e222a',
};

const soho = {
  0: '#ffffff',
  50: '#f4f4f4',
  100: '#e8e9e9',
  200: '#d2d2d4',
  300: '#bbbcbe',
  400: '#a5a5a9',
  500: '#8e8f93',
  600: '#77787d',
  700: '#616268',
  800: '#4a4b52',
  900: '#34343d',
  950: '#1d1e27',
};
const viva = {
  0: '#ffffff',
  50: '#f3f3f3',
  100: '#e7e7e8',
  200: '#cfd0d0',
  300: '#b7b8b9',
  400: '#9fa1a1',
  500: '#87898a',
  600: '#6e7173',
  700: '#565a5b',
  800: '#3e4244',
  900: '#262b2c',
  950: '#0e1315',
};
const ocean = {
  0: '#ffffff',
  50: '#fbfcfc',
  100: '#F7F9F8',
  200: '#EFF3F2',
  300: '#DADEDD',
  400: '#B1B7B6',
  500: '#828787',
  600: '#5F7274',
  700: '#415B61',
  800: '#29444E',
  900: '#183240',
  950: '#0c1920',
};

const DefaultTheme = definePreset(Aura, themeColors('blue', shark));
const CastleTheme = definePreset(Aura, themeColors('red', viva));
const OceanTheme = definePreset(Aura, themeColors('cyan', ocean));
const FlowerTheme = definePreset(Aura, themeColors('pink', soho));
const SunsetTheme = definePreset(Aura, themeColors('orange', 'stone'));
const MintTheme = definePreset(Aura, themeColors('green', 'slate'));
const MonochromeTheme = definePreset(Aura, themeColors('gray', 'neutral'));

export interface Theme {
  name: string;
  primengTheme: Preset;
}

export const themes = {
  default: {
    name: 'Default',
    primengTheme: DefaultTheme,
  },
  castle: {
    name: 'Castle',
    primengTheme: CastleTheme,
  },
  ocean: {
    name: 'Ocean',
    primengTheme: OceanTheme,
  },
  flower: {
    name: 'Flower',
    primengTheme: FlowerTheme,
  },
  sunset: {
    name: 'Sunset',
    primengTheme: SunsetTheme,
  },
  mint: {
    name: 'Mint',
    primengTheme: MintTheme,
  },
  monochrome: {
    name: 'Monochrome',
    primengTheme: MonochromeTheme,
  },
} as const;
export type PrimeVueThemeId = keyof typeof themes;
export const primeVueThemeIds = Object.keys(themes) as PrimeVueThemeId[];

export const darkModeOptions = ['light', 'dark', 'system'] as const;
export type DarkModeOption = (typeof darkModeOptions)[number];
