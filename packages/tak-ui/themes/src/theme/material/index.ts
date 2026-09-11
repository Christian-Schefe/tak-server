import {
  argbFromHex,
  DynamicScheme,
  Hct,
  hexFromArgb,
  Variant,
} from '@material/material-color-utilities';
import type { Theme } from '../..';
import { createButtonTokens, type ButtonTokens } from './button';
import type { CardSemantic } from './semantic/card';
import type { DialogSemantic } from './semantic/dialog';
import type { DropdownSemantic } from './semantic/dropdown';
import type { InputTextSemantic } from './semantic/inputtext';
import type { RootSemantic } from './semantic/root';
import type { ScrollbarSemantic } from './semantic/scrollbar';
import type { SelectSemantic } from './semantic/select';
import type { SideBarSemantic } from './semantic/sidebar';
import type { SliderSemantic } from './semantic/slider';
import type { TextSemantic } from './semantic/text';
import type { ToggleSemantic } from './semantic/toggle';
import type { TooltipSemantic } from './semantic/tooltip';

import './style.scss';

type LayoutScheme = {
  borderRadius: string;
  borderRadiusSmall: string;
};

const layoutScheme: LayoutScheme = {
  borderRadius: '0.5rem',
  borderRadiusSmall: '0.375rem',
};

export function createDefaultTheme(theme: DynamicScheme) {
  const defaultButtonSemantic: ButtonTokens = createButtonTokens(theme);

  const defaultCardSemantic: CardSemantic = {
    'border-radius': layoutScheme.borderRadius,
    padding: '1rem',
    gap: '1rem',
    background: hexFromArgb(theme.surfaceContainerLow),
    text: hexFromArgb(theme.onSurface),
    border: `none`,
    'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
  };

  const defaultDialogSemantic: DialogSemantic = {
    'border-radius': layoutScheme.borderRadius,
    'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
    background: hexFromArgb(theme.surface),
    padding: '1rem',
    'mask-background': 'rgba(0, 0, 0, 0.5)',
  };

  const defaultInputTextSemantic: InputTextSemantic = {
    normal: {
      background: hexFromArgb(theme.surfaceContainerHigh),
      'border-radius': layoutScheme.borderRadiusSmall,
      outline: '2px solid transparent',
      border: 'none',
      'text-empty': hexFromArgb(theme.onSurfaceVariant),
      'text-filled': hexFromArgb(theme.onSurface),
      label: {
        color: hexFromArgb(theme.onSurfaceVariant),
        top: '0.375rem',
      },
      icon: {
        padding: '0.75rem',
        color: hexFromArgb(theme.onSurfaceVariant),
      },
      padding: { left: '0.75rem', top: '0.25rem', right: '0.75rem', bottom: '0.25rem' },
      'padding-with-label': { top: '1rem', bottom: '0.25rem' },
      width: '16rem',
      height: '2.75rem',
      support: {
        padding: '0.25rem 0.75rem',
        color: hexFromArgb(theme.onSurfaceVariant),
      },
      opacity: '1',
    },
    hovered: {
      background: hexFromArgb(theme.surfaceContainerHighest),
    },
    focused: {
      background: hexFromArgb(theme.surfaceContainerHighest),
      outline: `2px solid ${hexFromArgb(theme.primary)}`,
      label: {
        color: hexFromArgb(theme.primary),
      },
      icon: {
        color: hexFromArgb(theme.primary),
      },
    },
    disabled: {
      opacity: '0.5',
    },
  };

  const defaultScrollbarSemantic: ScrollbarSemantic = {
    track: hexFromArgb(theme.surface),
    thumb: hexFromArgb(theme.outline),
  };

  const defaultTextSemantic: TextSemantic = {
    small: {
      size: '0.875rem',
      'line-height': 'calc(1.25rem / 0.875rem)',
    },
    medium: {
      size: '1rem',
      'line-height': 'calc(1.5rem / 1rem)',
    },
    large: {
      size: '1.25rem',
      'line-height': 'calc(1.75rem / 1.25rem)',
    },
  };

  const defaultRootSemantic: RootSemantic = {
    background: hexFromArgb(theme.surface),
    text: hexFromArgb(theme.onSurface),
  };
  const defaultSliderSemantic: SliderSemantic = {
    normal: {
      track: {
        padding: '0.5rem',
        gap: '0.75rem',
        height: '1.25rem',
        filled: {
          background: hexFromArgb(theme.primary),
        },
        unfilled: {
          background: hexFromArgb(theme.secondaryContainer),
        },
        'border-radius': '1.25rem',
        'border-radius-inner': '0.25rem',
      },
      handle: {
        width: '0.25rem',
        height: '2.5rem',
        background: hexFromArgb(theme.primary),
        'border-radius': '1rem',
      },
      opacity: '1',
    },
    hovered: {},
    pressed: {
      track: {
        gap: '0.375rem',
      },
      handle: {
        width: '0.125rem',
      },
    },
    disabled: {
      opacity: '0.5',
    },
    focus: {
      outline: `2px solid ${hexFromArgb(theme.outline)}`,
      'outline-offset': '2px',
    },
  };

  const defaultSelectSemantic: SelectSemantic = {
    normal: {
      background: hexFromArgb(theme.surfaceContainerHigh),
      'border-radius': layoutScheme.borderRadiusSmall,
      width: '16rem',
      height: '2.75rem',
      outline: `2px solid transparent`,
      label: {
        color: hexFromArgb(theme.onSurfaceVariant),
        top: '0.375rem',
      },
      'icon-padding': '0.25rem',
      'icon-color': hexFromArgb(theme.onSurfaceVariant),
      padding: { left: '0.75rem', top: '0.25rem', right: '0.75rem', bottom: '0.25rem' },
      'padding-with-label': { top: '0.875rem', bottom: '0.25rem' },
      'text-empty': hexFromArgb(theme.onSurfaceVariant),
      'text-filled': hexFromArgb(theme.onSurface),
      opacity: '1',
    },
    hovered: {
      background: hexFromArgb(theme.surfaceContainerHighest),
    },
    disabled: {
      opacity: '0.5',
    },
    focused: {
      background: hexFromArgb(theme.surfaceContainerHighest),
      outline: `2px solid ${hexFromArgb(theme.primary)}`,
      label: {
        color: hexFromArgb(theme.primary),
      },
      'icon-color': hexFromArgb(theme.primary),
    },
    dropdown: {
      background: hexFromArgb(theme.surfaceContainer),
      'border-radius': layoutScheme.borderRadius,
      'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
      padding: '0.5rem',
    },
  };

  const defaultSideBarSemantic: SideBarSemantic = {
    background: hexFromArgb(theme.surfaceContainer),
    text: hexFromArgb(theme.onSurface),
    border: 'none',
    padding: '0.5rem',
    'mask-background': 'rgba(0, 0, 0, 0.5)',
  };

  const defaultTooltipSemantic: TooltipSemantic = {
    background: hexFromArgb(theme.inverseSurface),
    text: hexFromArgb(theme.inverseOnSurface),
    'border-radius': layoutScheme.borderRadius,
    'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
    padding: '0.5rem',
  };

  const defaultToggleSemantic: ToggleSemantic = {
    normal: {
      off: {
        track: {
          background: hexFromArgb(theme.surfaceContainerHighest),
          border: `2px solid ${hexFromArgb(theme.outline)}`,
          'border-radius': '1.5rem',
        },
        handle: {
          width: '0.875rem',
          height: '0.875rem',
          background: hexFromArgb(theme.outline),
          'border-radius': '1rem',
        },
      },
      on: {
        track: {
          background: hexFromArgb(theme.primary),
          border: '2px solid transparent',
          'border-radius': '1.5rem',
        },
        handle: {
          width: '1.25rem',
          height: '1.25rem',
          background: hexFromArgb(theme.onPrimary),
          'border-radius': '1rem',
        },
      },
      opacity: '1',
      track: {
        width: '3rem',
        height: '1.875rem',
      },
    },
    hovered: {},
    pressed: {
      off: {
        handle: {
          width: '1.25rem',
          height: '1.25rem',
        },
      },
      on: {
        handle: {
          width: '1.5rem',
          height: '1.5rem',
        },
      },
    },
    disabled: {
      opacity: '0.5',
    },
    focus: {
      outline: `2px solid ${hexFromArgb(theme.outline)}`,
      'outline-offset': '2px',
    },
  };

  const defaultDropdownSemantic: DropdownSemantic = {
    'transform-enter-from': 'scale(0.9)',
  };

  return {
    root: defaultRootSemantic,
    text: defaultTextSemantic,
    button: defaultButtonSemantic,
    card: defaultCardSemantic,
    dialog: defaultDialogSemantic,
    inputtext: defaultInputTextSemantic,
    scrollbar: defaultScrollbarSemantic,
    slider: defaultSliderSemantic,
    select: defaultSelectSemantic,
    sidebar: defaultSideBarSemantic,
    tooltip: defaultTooltipSemantic,
    toggle: defaultToggleSemantic,
    dropdown: defaultDropdownSemantic,
  };
}

export const materialTheme: Theme = createMaterialTheme('#6750A4');

export function createMaterialTheme(sourceColor: string): Theme {
  return {
    light: createDefaultTheme(createMaterialDynamicTheme(sourceColor, false)),
    dark: createDefaultTheme(createMaterialDynamicTheme(sourceColor, true)),
  };
}

export function createMaterialDynamicTheme(sourceColor: string, isDark: boolean): DynamicScheme {
  const themeColor = Hct.fromInt(argbFromHex(sourceColor));
  const theme = new DynamicScheme({
    sourceColorHct: themeColor,
    variant: Variant.TONAL_SPOT,
    contrastLevel: 0,
    isDark,
  });
  return theme;
}
