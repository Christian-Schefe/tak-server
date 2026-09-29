import {
  argbFromHex,
  DynamicScheme,
  Hct,
  hexFromArgb,
  Variant,
} from '@material/material-color-utilities';
import { createButtonTokens } from './button';
import { createTooltipTokens } from './tooltip';

import { createCardTokens } from './card';
import { createDialogTokens } from './dialog';
import { createDropdownTokens } from './dropdown';
import { createInputTextTokens } from './inputtext';
import { createRootTokens } from './root';
import { createSideBarTokens } from './sidebar';
import { createSliderTokens } from './slider';
import './style.scss';
import { createToggleTokens } from './toggle';
import { createLabelFieldTokens } from './labelfield';
import { createSelectTextTokens } from './select';
import { createBadgeTokens } from './badge';

function createDefaultTheme(theme: DynamicScheme) {
  return {
    color: {
      primary: hexFromArgb(theme.primary),
      onPrimary: hexFromArgb(theme.onPrimary),
      primaryContainer: hexFromArgb(theme.primaryContainer),
      onPrimaryContainer: hexFromArgb(theme.onPrimaryContainer),
      secondary: hexFromArgb(theme.secondary),
      onSecondary: hexFromArgb(theme.onSecondary),
      secondaryContainer: hexFromArgb(theme.secondaryContainer),
      onSecondaryContainer: hexFromArgb(theme.onSecondaryContainer),
      tertiary: hexFromArgb(theme.tertiary),
      onTertiary: hexFromArgb(theme.onTertiary),
      tertiaryContainer: hexFromArgb(theme.tertiaryContainer),
      onTertiaryContainer: hexFromArgb(theme.onTertiaryContainer),
      error: hexFromArgb(theme.error),
      onError: hexFromArgb(theme.onError),
      errorContainer: hexFromArgb(theme.errorContainer),
      onErrorContainer: hexFromArgb(theme.onErrorContainer),
      background: hexFromArgb(theme.background),
      onBackground: hexFromArgb(theme.onBackground),
      surface: hexFromArgb(theme.surface),
      onSurface: hexFromArgb(theme.onSurface),
      surfaceVariant: hexFromArgb(theme.surfaceVariant),
      onSurfaceVariant: hexFromArgb(theme.onSurfaceVariant),
      outline: hexFromArgb(theme.outline),
      outlineVariant: hexFromArgb(theme.outlineVariant),
      shadow: hexFromArgb(theme.shadow),
      scrim: hexFromArgb(theme.scrim),
      inverseSurface: hexFromArgb(theme.inverseSurface),
      inverseOnSurface: hexFromArgb(theme.inverseOnSurface),
      inversePrimary: hexFromArgb(theme.inversePrimary),
    },
    root: createRootTokens(theme),
    button: createButtonTokens(theme),
    card: createCardTokens(theme),
    dialog: createDialogTokens(theme),
    inputtext: createInputTextTokens(theme),
    slider: createSliderTokens(theme),
    sidebar: createSideBarTokens(theme),
    tooltip: createTooltipTokens(theme),
    toggle: createToggleTokens(theme),
    dropdown: createDropdownTokens(),
    labelfield: createLabelFieldTokens(theme),
    select: createSelectTextTokens(theme),
    badge: createBadgeTokens(theme),
  };
}

export type Theme = {
  light: unknown;
  dark: unknown;
};

export const materialTheme: Theme = createMaterialTheme('#6750A4');

type MaterialThemeMode =
  | 'tonal'
  | 'monochrome'
  | 'vibrant'
  | 'expressive'
  | 'rainbow'
  | 'fruit_salad';

function themeModeToVariant(mode: MaterialThemeMode): Variant {
  switch (mode) {
    case 'tonal':
      return Variant.TONAL_SPOT;
    case 'monochrome':
      return Variant.MONOCHROME;
    case 'vibrant':
      return Variant.VIBRANT;
    case 'expressive':
      return Variant.EXPRESSIVE;
    case 'rainbow':
      return Variant.RAINBOW;
    case 'fruit_salad':
      return Variant.FRUIT_SALAD;
  }
}

export function createMaterialTheme(sourceColor: string, mode: MaterialThemeMode = 'tonal'): Theme {
  return {
    light: createDefaultTheme(createMaterialDynamicTheme(sourceColor, false, mode)),
    dark: createDefaultTheme(createMaterialDynamicTheme(sourceColor, true, mode)),
  };
}

function createMaterialDynamicTheme(
  sourceColor: string,
  isDark: boolean,
  mode: MaterialThemeMode,
): DynamicScheme {
  const variant = themeModeToVariant(mode);
  const themeColor = Hct.fromInt(argbFromHex(sourceColor));
  const theme = new DynamicScheme({
    sourceColorHct: themeColor,
    specVersion: '2025',
    variant,
    contrastLevel: 0,
    isDark,
  });
  return theme;
}
