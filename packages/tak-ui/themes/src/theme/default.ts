import type { FullTheme } from '.';
import type { ButtonSemantic } from '../semantic/button';
import type { CardSemantic } from '../semantic/card';
import type { ColorsSemantic } from '../semantic/colors';
import type { DialogSemantic } from '../semantic/dialog';
import type { InputTextSemantic } from '../semantic/inputtext';
import type { ScrollbarSemantic } from '../semantic/scrollbar';
import type { SelectSemantic } from '../semantic/select';
import type { SideBarSemantic } from '../semantic/sidebar';
import type { SliderSemantic } from '../semantic/slider';
import type { TextSemantic } from '../semantic/text';
import type { ToggleSemantic } from '../semantic/toggle';
import type { TooltipSemantic } from '../semantic/tooltip';
import { blue, soho } from './colors';

const primaryColor = '{primary-500|primary-400}';
const primaryColorHover = '{primary-600|primary-300}';
const primaryColorActive = '{primary-700|primary-200}';

const textOnPrimary = '{surface-0|surface-950}';

const secondaryColor = '{surface-100|surface-700}';
const secondaryColorHover = '{surface-200|surface-600}';
const secondaryColorActive = '{surface-300|surface-500}';

const textOnSecondary = '{surface-700|surface-200}';

const focusOutline = '2px solid {surface-700|surface-200}';

const border = '1px solid {surface-200|surface-700}';

const defaultButtonSemantic: ButtonSemantic = {
  normal: {
    'border-radius': '0.25rem',
    size: '1.5rem',
    padding: '0.5rem',
    gap: '0.5rem',
    opacity: '1',
    filled: {
      secondary: {
        background: secondaryColor,
        text: textOnSecondary,
        border: 'none',
      },
      primary: {
        background: primaryColor,
        text: textOnPrimary,
        border: 'none',
      },
    },
    text: {
      secondary: {
        background: 'transparent',
        text: textOnSecondary,
        border: 'none',
      },
      primary: {
        background: 'transparent',
        text: primaryColor,
        border: 'none',
      },
    },
    outlined: {
      secondary: {
        background: 'transparent',
        text: textOnSecondary,
        border: border,
      },
      primary: {
        background: 'transparent',
        text: primaryColor,
        border: `1px solid ${primaryColor}`,
      },
    },
  },
  hovered: {
    filled: {
      secondary: {
        background: secondaryColorHover,
      },
      primary: {
        background: primaryColorHover,
      },
    },
    text: {
      secondary: {
        background: `color-mix(in srgb, ${secondaryColorHover} 10%, transparent)`,
      },
      primary: {
        background: `color-mix(in srgb, ${primaryColorHover} 10%, transparent)`,
      },
    },
    outlined: {
      secondary: {
        background: `color-mix(in srgb, ${secondaryColorHover} 10%, transparent)`,
      },
      primary: {
        background: `color-mix(in srgb, ${primaryColorHover} 10%, transparent)`,
      },
    },
  },
  pressed: {
    filled: {
      secondary: {
        background: secondaryColorActive,
      },
      primary: {
        background: primaryColorActive,
      },
    },
    text: {
      secondary: {
        background: `color-mix(in srgb, ${secondaryColorActive} 10%, transparent)`,
      },
      primary: {
        background: `color-mix(in srgb, ${primaryColorActive} 10%, transparent)`,
      },
    },
    outlined: {
      secondary: {
        background: `color-mix(in srgb, ${secondaryColorActive} 10%, transparent)`,
      },
      primary: {
        background: `color-mix(in srgb, ${primaryColorActive} 10%, transparent)`,
      },
    },
  },
  disabled: {
    opacity: '0.5',
  },
  focus: {
    outline: focusOutline,
    'outline-offset': '2px',
  },
};

const defaultCardSemantic: CardSemantic = {
  'border-radius': '0.25rem',
  padding: '1rem',
  gap: '1rem',
  background: '{surface-0|surface-900}',
  border: border,
};

const defaultDialogSemantic: DialogSemantic = {
  'border-radius': '0.25rem',
  'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
  background: '{surface-0|surface-900}',
  padding: '1rem',
  'mask-background': 'rgba(0, 0, 0, 0.5)',
};

const defaultInputTextSemantic: InputTextSemantic = {
  normal: {
    background: '{surface-0|surface-900}',
    'border-radius': '0.25rem',
    outline: border,
    'text-empty': '{surface-500|surface-400}',
    'text-filled': '{surface-950|surface-0}',
    label: {
      color: '{surface-500|surface-400}',
      padding: '0.25rem',
    },
    padding: { x: '0.75rem', y: '0.5rem' },
    width: '16rem',
    support: {
      padding: '0.25rem 0.75rem',
      color: '{surface-500|surface-400}',
    },
  },
  hovered: {
    outline: '1px solid {surface-400|surface-500}',
  },
  focused: {
    outline: `2px solid ${primaryColor}`,
    label: {
      color: primaryColor,
    },
  },
};

const defaultScrollbarSemantic: ScrollbarSemantic = {
  track: '{surface-0|surface-900}',
  thumb: '{surface-400|surface-500}',
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

const defaultColorsSemantic: ColorsSemantic = {
  background: '{surface-50|surface-950}',
  text: '{surface-950|surface-0}',
};
const defaultSliderSemantic: SliderSemantic = {
  normal: {
    track: {
      padding: '0.5rem',
      gap: '0.75rem',
      height: '1.25rem',
      filled: {
        background: primaryColor,
      },
      unfilled: {
        background: secondaryColor,
      },
      'border-radius': '0.25rem',
    },
    handle: {
      width: '0.25rem',
      height: '2.5rem',
      background: primaryColor,
      'border-radius': '1rem',
    },
    opacity: '1',
  },
  hovered: {
    track: {
      filled: {
        background: primaryColorHover,
      },
      unfilled: {
        background: '{surface-300|surface-600}',
      },
    },
    handle: {
      background: primaryColorHover,
    },
  },
  pressed: {
    track: {
      gap: '0.375rem',
      filled: {
        background: primaryColorHover,
      },
      unfilled: {
        background: '{surface-300|surface-600}',
      },
    },
    handle: {
      width: '0.125rem',
      background: primaryColorHover,
    },
  },
  disabled: {
    opacity: '0.5',
  },
  focus: {
    outline: focusOutline,
    'outline-offset': '2px',
  },
};

const defaultSelectSemantic: SelectSemantic = {
  background: '{surface-0|surface-900}',
  'border-radius': '0.25rem',
  border: border,
  'border-hover': '1px solid {surface-400|surface-500}',
  'border-focus': `1px solid ${primaryColor}`,
  padding: { x: '0.5rem', y: '0.25rem' },
  dropdown: {
    background: '{surface-0|surface-900}',
    'border-radius': '0.25rem',
    'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
    padding: '0.5rem',
  },
  'filled-text': '{surface-950|surface-0}',
  'empty-text': '{surface-500|surface-400}',
  'icon-color': '{surface-500|surface-400}',
  'label-text-focus': primaryColor,
};

const defaultSideBarSemantic: SideBarSemantic = {
  background: '{surface-0|surface-900}',
  border: border,
  padding: '0.5rem',
  'mask-background': 'rgba(0, 0, 0, 0.5)',
};

const defaultTooltipSemantic: TooltipSemantic = {
  background: '{surface-900|surface-0}',
  text: '{surface-0|surface-900}',
  'border-radius': '0.25rem',
  'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
  padding: '0.5rem',
};

const defaultToggleSemantic: ToggleSemantic = {
  normal: {
    off: {
      track: {
        background: secondaryColor,
        border: `2px solid ${textOnSecondary}`,
        'border-radius': '1.5rem',
      },
      handle: {
        width: '0.875rem',
        height: '0.875rem',
        background: textOnSecondary,
        'border-radius': '1rem',
      },
    },
    on: {
      track: {
        background: primaryColor,
        border: '2px solid transparent',
        'border-radius': '1.5rem',
      },
      handle: {
        width: '1.25rem',
        height: '1.25rem',
        background: '{surface-0|surface-900}',
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
        background: secondaryColorHover,
      },
    },
    on: {
      track: {
        background: primaryColorHover,
      },
    },
  },
  pressed: {
    off: {
      track: {
        background: secondaryColorHover,
      },
      handle: {
        width: '1.25rem',
        height: '1.25rem',
      },
    },
    on: {
      track: {
        background: primaryColorHover,
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
    outline: focusOutline,
    'outline-offset': '2px',
  },
};

export const defaultTheme: FullTheme = {
  id: 'default',
  colors: {
    primary: blue,
    surface: soho,
  },
  semantic: {
    color: defaultColorsSemantic,
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
