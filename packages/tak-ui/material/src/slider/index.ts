import { hexFromArgb, type DynamicScheme } from '@material/material-color-utilities';

export type SliderTokens = {
  track: {
    padding: string;
    gap: string;
    'pressed-gap': string;
    height: string;
    'border-radius': string;
    'border-radius-inner': string;
    'unfilled-background': string;
    'filled-background': string;
  };
  handle: {
    width: string;
    'pressed-width': string;
    height: string;
    background: string;
    'border-radius': string;
  };
  'disabled-opacity': string;
  'focus-outline': string;
  'focus-outline-offset': string;
};

export function createSliderTokens(theme: DynamicScheme): SliderTokens {
  return {
    track: {
      padding: '0.5rem',
      gap: '0.75rem',
      'pressed-gap': '0.375rem',
      height: '1.25rem',
      'filled-background': hexFromArgb(theme.primary),
      'unfilled-background': hexFromArgb(theme.secondaryContainer),
      'border-radius': '1.25rem',
      'border-radius-inner': '0.25rem',
    },
    handle: {
      width: '0.25rem',
      'pressed-width': '0.125rem',
      height: '2.5rem',
      background: hexFromArgb(theme.primary),
      'border-radius': '1rem',
    },
    'disabled-opacity': '0.5',
    'focus-outline': `2px solid ${hexFromArgb(theme.primary)}`,
    'focus-outline-offset': '2px',
  };
}
