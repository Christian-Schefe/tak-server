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

const defaultButtonSemantic: ButtonSemantic = {
  'border-radius': '0.25rem',
  size: '1.5rem',
  padding: '0.5rem',
  gap: '0.5rem',
  'focus-outline-offset': '2px',
  'focus-outline': '2px solid {surface-700|surface-200}',
  filled: {
    secondary: {
      background: '{surface-100|surface-700}',
      'disabled-background': '{surface-200|surface-600}',
      text: '{surface-700|surface-200}',
      'disabled-text': '{surface-500|surface-300}',
      hover: '{surface-200|surface-600}',
      active: '{surface-300|surface-500}',
      border: 'none',
    },
    primary: {
      background: '{primary-500|primary-400}',
      'disabled-background': '{primary-400|primary-500}',
      text: '{surface-0|surface-950}',
      'disabled-text': '{surface-50|surface-900}',
      hover: '{primary-600|primary-300}',
      active: '{primary-700|primary-200}',
      border: 'none',
    },
  },
  text: {
    secondary: {
      background: 'transparent',
      'disabled-background': 'color-mix(in srgb, {surface-200|surface-600} 10%, transparent)',
      text: '{surface-700|surface-200}',
      'disabled-text': '{surface-500|surface-200}',
      hover: 'color-mix(in srgb, {surface-200|surface-600} 10%, transparent)',
      active: 'color-mix(in srgb, {surface-300|surface-500} 10%, transparent)',
      border: 'none',
    },
    primary: {
      background: 'transparent',
      'disabled-background': 'color-mix(in srgb, {primary-600|primary-300} 10%, transparent)',
      text: '{primary-500|primary-400}',
      'disabled-text': '{primary-400|primary-500}',
      hover: 'color-mix(in srgb, {primary-600|primary-300} 10%, transparent)',
      active: 'color-mix(in srgb, {primary-700|primary-200} 10%, transparent)',
      border: 'none',
    },
  },
  outlined: {
    secondary: {
      background: 'transparent',
      'disabled-background': 'color-mix(in srgb, {surface-200|surface-600} 10%, transparent)',
      text: '{surface-700|surface-200}',
      'disabled-text': '{surface-600|surface-300}',
      hover: 'color-mix(in srgb, {surface-200|surface-600} 10%, transparent)',
      active: 'color-mix(in srgb, {surface-300|surface-500} 10%, transparent)',
      border: '1px solid {surface-200|surface-700}',
    },
    primary: {
      background: 'transparent',
      'disabled-background': 'color-mix(in srgb, {primary-600|primary-300} 10%, transparent)',
      text: '{primary-500|primary-400}',
      'disabled-text': '{primary-400|primary-500}',
      hover: 'color-mix(in srgb, {primary-600|primary-300} 10%, transparent)',
      active: 'color-mix(in srgb, {primary-700|primary-200} 10%, transparent)',
      border: '1px solid {primary-500|primary-400}',
    },
  },
};

const defaultCardSemantic: CardSemantic = {
  'border-radius': '0.25rem',
  padding: '1rem',
  gap: '1rem',
  background: '{surface-0|surface-900}',
  border: '1px solid {surface-200|surface-700}',
};

const defaultDialogSemantic: DialogSemantic = {
  'border-radius': '0.25rem',
  'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
  background: '{surface-0|surface-900}',
  padding: '1rem',
  'mask-background': 'rgba(0, 0, 0, 0.5)',
};

const defaultInputTextSemantic: InputTextSemantic = {
  background: '{surface-0|surface-900}',
  'border-radius': '0.25rem',
  border: '1px solid {surface-200|surface-700}',
  'border-hover': '1px solid {surface-400|surface-500}',
  'border-focus': '1px solid {primary-500|primary-400}',
  'empty-text': '{surface-500|surface-400}',
  'filled-text': '{surface-950|surface-0}',
  padding: { x: '0.5rem', y: '0.25rem' },
  width: '16rem',
  'label-text-focus': '{primary-500|primary-400}',
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
  height: '1.25rem',
  gap: '0.5rem',
  'border-radius': '0.25rem',
  padding: '0.5rem',
  handle: {
    width: '0.25rem',
    height: '2.5rem',
    background: '{primary-500|primary-400}',
    'hover-background': '{primary-600|primary-300}',
    border: 'none',
    'border-radius': '0.125rem',
    'disabled-background': '{primary-400|primary-500}',
    'focus-outline': '2px solid {surface-700|surface-200}',
    'focus-outline-offset': '2px',
  },
  background: '{surface-200|surface-700}',
  'hover-background': '{surface-300|surface-600}',
  'disabled-background': '{surface-300|surface-600}',
  'filled-background': '{primary-500|primary-400}',
  'hover-filled-background': '{primary-600|primary-300}',
  'disabled-filled-background': '{primary-400|primary-500}',
};

const defaultSelectSemantic: SelectSemantic = {
  background: '{surface-0|surface-900}',
  'border-radius': '0.25rem',
  border: '1px solid {surface-200|surface-700}',
  'border-hover': '1px solid {surface-400|surface-500}',
  'border-focus': '1px solid {primary-500|primary-400}',
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
  'label-text-focus': '{primary-500|primary-400}',
};

const defaultSideBarSemantic: SideBarSemantic = {
  background: '{surface-0|surface-900}',
  border: '1px solid {surface-200|surface-700}',
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
  width: '2.5rem',
  height: '1.5rem',
  background: '{surface-200|surface-700}',
  'hover-background': '{surface-300|surface-600}',
  'disabled-background': '{surface-300|surface-600}',
  'active-background': '{primary-500|primary-400}',
  'active-hover-background': '{primary-600|primary-300}',
  'disabled-active-background': '{primary-400|primary-500}',
  'focus-outline': '2px solid {surface-700|surface-200}',
  'focus-outline-offset': '2px',
  handle: {
    size: '1rem',
    background: '{surface-0|surface-300}',
    'hover-background': '{surface-0|surface-200}',
    'disabled-background': '{surface-100|surface-400}',
    'active-background': '{surface-0|surface-900}',
    'active-hover-background': '{surface-0|surface-900}',
    'active-disabled-background': '{surface-100|surface-800}',
    border: 'none',
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
