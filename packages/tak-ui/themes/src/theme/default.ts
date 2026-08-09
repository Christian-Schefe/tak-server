import type { FullTheme } from '.';
import type { ButtonSemantic } from '../semantic/button';
import type { CardSemantic } from '../semantic/card';
import type { DialogSemantic } from '../semantic/dialog';
import type { InputTextSemantic } from '../semantic/inputtext';
import type { RootSemantic } from '../semantic/root';
import type { ScrollbarSemantic } from '../semantic/scrollbar';
import type { SelectSemantic } from '../semantic/select';
import type { SideBarSemantic } from '../semantic/sidebar';
import type { SliderSemantic } from '../semantic/slider';
import type { TextSemantic } from '../semantic/text';
import type { ToggleSemantic } from '../semantic/toggle';
import type { TooltipSemantic } from '../semantic/tooltip';
import { materialColorScheme, type ColorScheme } from './schemas';

type LayoutScheme = {
  borderRadius: string;
  borderRadiusSmall: string;
};

const layoutScheme: LayoutScheme = {
  borderRadius: '0.75rem',
  borderRadiusSmall: '0.5rem',
};

export function createDefaultTheme(colorScheme: ColorScheme): FullTheme {
  const defaultButtonSemantic: ButtonSemantic = {
    normal: {
      'border-radius': layoutScheme.borderRadiusSmall,
      size: '1.5rem',
      padding: '0.5rem',
      gap: '0.5rem',
      opacity: '1',
      filled: {
        secondary: {
          background: colorScheme.secondaryContainer,
          text: colorScheme.onSecondaryContainer,
          border: 'none',
        },
        primary: {
          background: colorScheme.primary,
          text: colorScheme.onPrimary,
          border: 'none',
        },
      },
      text: {
        secondary: {
          background: 'transparent',
          text: colorScheme.secondary,
          border: 'none',
        },
        primary: {
          background: 'transparent',
          text: colorScheme.primary,
          border: 'none',
        },
      },
      outlined: {
        secondary: {
          background: 'transparent',
          text: colorScheme.secondary,
          border: `1px solid ${colorScheme.secondary}`,
        },
        primary: {
          background: 'transparent',
          text: colorScheme.primary,
          border: `1px solid ${colorScheme.primary}`,
        },
      },
    },
    hovered: {
      filled: {
        secondary: {
          background: colorScheme.secondaryContainerHover,
        },
        primary: {
          background: colorScheme.primaryHover,
        },
      },
      text: {
        secondary: {
          background: `color-mix(in srgb, ${colorScheme.secondaryContainerHover} 10%, transparent)`,
        },
        primary: {
          background: `color-mix(in srgb, ${colorScheme.primaryHover} 10%, transparent)`,
        },
      },
      outlined: {
        secondary: {
          background: `color-mix(in srgb, ${colorScheme.secondaryContainerHover} 10%, transparent)`,
        },
        primary: {
          background: `color-mix(in srgb, ${colorScheme.primaryHover} 10%, transparent)`,
        },
      },
    },
    pressed: {
      filled: {
        secondary: {
          background: colorScheme.secondaryContainerActive,
        },
        primary: {
          background: colorScheme.primaryActive,
        },
      },
      text: {
        secondary: {
          background: `color-mix(in srgb, ${colorScheme.secondary} 10%, transparent)`,
        },
        primary: {
          background: `color-mix(in srgb, ${colorScheme.primary} 10%, transparent)`,
        },
      },
      outlined: {
        secondary: {
          background: `color-mix(in srgb, ${colorScheme.secondary} 10%, transparent)`,
        },
        primary: {
          background: `color-mix(in srgb, ${colorScheme.primary} 10%, transparent)`,
        },
      },
    },
    disabled: {
      opacity: '0.5',
    },
    focus: {
      outline: `2px solid ${colorScheme.outline}`,
      'outline-offset': '2px',
    },
  };

  const defaultCardSemantic: CardSemantic = {
    'border-radius': layoutScheme.borderRadius,
    padding: '1rem',
    gap: '1rem',
    background: colorScheme.surfaceContainer,
    text: colorScheme.onSurfaceContainer,
    border: `none`,
  };

  const defaultDialogSemantic: DialogSemantic = {
    'border-radius': layoutScheme.borderRadius,
    'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
    background: colorScheme.surface,
    padding: '1rem',
    'mask-background': 'rgba(0, 0, 0, 0.5)',
  };

  const defaultInputTextSemantic: InputTextSemantic = {
    normal: {
      background: 'transparent',
      'border-radius': layoutScheme.borderRadiusSmall,
      outline: `1px solid ${colorScheme.outline}`,
      'text-empty': colorScheme.onSurfaceVariant,
      'text-filled': colorScheme.onSurface,
      label: {
        color: colorScheme.onSurfaceVariant,
        top: '0.375rem',
      },
      'icon-padding': '0.25rem',
      padding: { left: '0.75rem', top: '0.875rem', right: '0.75rem', bottom: '0.25rem' },
      width: '16rem',
      height: '2.75rem',
      support: {
        padding: '0.25rem 0.75rem',
        color: colorScheme.onSurfaceVariant,
      },
      opacity: '1',
    },
    hovered: {
      outline: `1px solid ${colorScheme.outline}`,
    },
    focused: {
      outline: `2px solid ${colorScheme.primary}`,
      label: {
        color: colorScheme.primary,
      },
    },
    disabled: {
      opacity: '0.5',
    },
  };

  const defaultScrollbarSemantic: ScrollbarSemantic = {
    track: colorScheme.surface,
    thumb: colorScheme.outline,
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
    background: colorScheme.surface,
    text: colorScheme.onSurface,
  };
  const defaultSliderSemantic: SliderSemantic = {
    normal: {
      track: {
        padding: '0.5rem',
        gap: '0.75rem',
        height: '1.25rem',
        filled: {
          background: colorScheme.primary,
        },
        unfilled: {
          background: colorScheme.secondaryContainer,
        },
        'border-radius': '1.25rem',
      },
      handle: {
        width: '0.25rem',
        height: '2.5rem',
        background: colorScheme.primary,
        'border-radius': '1rem',
      },
      opacity: '1',
    },
    hovered: {
      track: {
        filled: {
          background: colorScheme.primaryHover,
        },
        unfilled: {
          background: colorScheme.secondaryContainerHover,
        },
      },
      handle: {
        background: colorScheme.primaryHover,
      },
    },
    pressed: {
      track: {
        gap: '0.375rem',
        filled: {
          background: colorScheme.primaryHover,
        },
        unfilled: {
          background: colorScheme.secondaryContainerHover,
        },
      },
      handle: {
        width: '0.125rem',
        background: colorScheme.primaryHover,
      },
    },
    disabled: {
      opacity: '0.5',
    },
    focus: {
      outline: `2px solid ${colorScheme.outline}`,
      'outline-offset': '2px',
    },
  };

  const defaultSelectSemantic: SelectSemantic = {
    normal: {
      background: 'transparent',
      'border-radius': layoutScheme.borderRadiusSmall,
      width: '16rem',
      height: '2.75rem',
      outline: `1px solid ${colorScheme.outline}`,
      label: {
        color: colorScheme.onSurfaceVariant,
        top: '0.375rem',
      },
      'icon-padding': '0.25rem',
      'icon-color': colorScheme.onSurfaceVariant,
      padding: { left: '0.75rem', top: '0.875rem', right: '0.75rem', bottom: '0.25rem' },

      'text-empty': colorScheme.onSurfaceVariant,
      'text-filled': colorScheme.onSurface,
      opacity: '1',
    },
    hovered: {
      outline: `1px solid ${colorScheme.outline}`,
    },
    disabled: {
      opacity: '0.5',
    },
    focused: {
      outline: `2px solid ${colorScheme.primary}`,
      label: {
        color: colorScheme.primary,
      },
      'icon-color': colorScheme.primary,
    },
    dropdown: {
      background: colorScheme.surface,
      'border-radius': layoutScheme.borderRadius,
      'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
      padding: '0.5rem',
    },
  };

  const defaultSideBarSemantic: SideBarSemantic = {
    background: colorScheme.surfaceContainer,
    text: colorScheme.onSurfaceContainer,
    border: 'none',
    padding: '0.5rem',
    'mask-background': 'rgba(0, 0, 0, 0.5)',
  };

  const defaultTooltipSemantic: TooltipSemantic = {
    background: colorScheme.inverseSurface,
    text: colorScheme.onInverseSurface,
    'border-radius': layoutScheme.borderRadius,
    'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
    padding: '0.5rem',
  };

  const defaultToggleSemantic: ToggleSemantic = {
    normal: {
      off: {
        track: {
          background: colorScheme.surfaceContainer,
          border: `2px solid ${colorScheme.outline}`,
          'border-radius': '1.5rem',
        },
        handle: {
          width: '0.875rem',
          height: '0.875rem',
          background: colorScheme.outline,
          'border-radius': '1rem',
        },
      },
      on: {
        track: {
          background: colorScheme.primary,
          border: '2px solid transparent',
          'border-radius': '1.5rem',
        },
        handle: {
          width: '1.25rem',
          height: '1.25rem',
          background: colorScheme.onPrimary,
          'border-radius': '1rem',
        },
      },
      opacity: '1',
      track: {
        width: '3rem',
        height: '1.875rem',
      },
    },
    hovered: {
      off: {
        track: {
          background: colorScheme.secondaryContainerHover,
        },
      },
      on: {
        track: {
          background: colorScheme.primaryHover,
        },
      },
    },
    pressed: {
      off: {
        track: {
          background: colorScheme.secondaryContainerHover,
        },
        handle: {
          width: '1.25rem',
          height: '1.25rem',
        },
      },
      on: {
        track: {
          background: colorScheme.primaryHover,
        },
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
      outline: `2px solid ${colorScheme.outline}`,
      'outline-offset': '2px',
    },
  };
  return {
    id: 'default',
    semantic: {
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
    },
  };
}
export const defaultTheme: FullTheme = createDefaultTheme(materialColorScheme);
