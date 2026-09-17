import { hexFromArgb, type DynamicScheme } from '@material/material-color-utilities';

export type ToggleTokens = {
  track: {
    width: string;
    height: string;
    'border-radius': string;
  } & Record<'off' | 'on', { background: string; border: string }>;
  handle: {
    'border-radius': string;
  } & Record<
    'off' | 'on',
    {
      size: string;
      'pressed-size': string;
      background: string;
    }
  >;
  'disabled-opacity': string;
  'focus-outline': string;
  'focus-outline-offset': string;
};

export function createToggleTokens(theme: DynamicScheme): ToggleTokens {
  return {
    track: {
      width: '3rem',
      height: '1.875rem',
      'border-radius': '1.5rem',
      off: {
        background: hexFromArgb(theme.surfaceContainerHighest),
        border: `2px solid ${hexFromArgb(theme.outline)}`,
      },
      on: {
        background: hexFromArgb(theme.primary),
        border: '2px solid transparent',
      },
    },
    handle: {
      'border-radius': '1rem',
      off: {
        size: '0.875rem',
        'pressed-size': '1.25rem',
        background: hexFromArgb(theme.outline),
      },
      on: {
        size: '1.25rem',
        'pressed-size': '1.5rem',
        background: hexFromArgb(theme.onPrimary),
      },
    },
    'disabled-opacity': '0.5',
    'focus-outline': `2px solid ${hexFromArgb(theme.primary)}`,
    'focus-outline-offset': '2px',
  };
}
