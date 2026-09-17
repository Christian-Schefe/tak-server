import { argbFromHex, DynamicScheme, Hct, Variant } from '@material/material-color-utilities';
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

function createDefaultTheme(theme: DynamicScheme) {
  return {
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
  };
}

export type Theme = {
  light: unknown;
  dark: unknown;
};

export const materialTheme: Theme = createMaterialTheme('#6750A4');

export function createMaterialTheme(sourceColor: string): Theme {
  return {
    light: createDefaultTheme(createMaterialDynamicTheme(sourceColor, false)),
    dark: createDefaultTheme(createMaterialDynamicTheme(sourceColor, true)),
  };
}

function createMaterialDynamicTheme(sourceColor: string, isDark: boolean): DynamicScheme {
  const themeColor = Hct.fromInt(argbFromHex(sourceColor));
  const theme = new DynamicScheme({
    sourceColorHct: themeColor,
    variant: Variant.TONAL_SPOT,
    contrastLevel: 0,
    isDark,
  });
  return theme;
}
