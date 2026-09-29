import { hexFromArgb, type DynamicScheme } from '@material/material-color-utilities';

export type ButtonVariant = 'filled' | 'text' | 'outlined' | 'tonal';
export type ButtonSeverity = 'primary' | 'secondary' | 'tertiary' | 'danger';
export type ButtonSize = 'small' | 'medium';

export type ButtonTokens = {
  'hover-state-opacity': string;
  'pressed-state-opacity': string;
  'disabled-opacity': string;
  'focus-outline': string;
  'focus-outline-offset': string;
} & Record<
  ButtonVariant,
  Record<
    ButtonSeverity,
    {
      background: string;
      text: string;
      border: string;
      'state-color': string;
    }
  >
> &
  Record<
    ButtonSize,
    { padding: string; 'padding-icon-only': string; gap: string; 'font-size': string }
  >;

export function createButtonTokens(theme: DynamicScheme): ButtonTokens {
  return {
    'hover-state-opacity': '0.1',
    'pressed-state-opacity': '0.2',
    'disabled-opacity': '0.5',
    'focus-outline': `2px solid ${hexFromArgb(theme.primary)}`,
    'focus-outline-offset': '2px',
    small: {
      padding: '0.625rem 1rem',
      'padding-icon-only': '0.625rem',
      gap: '0.5rem',
      'font-size': '0.875rem',
    },
    medium: {
      padding: '1rem 1.5rem',
      'padding-icon-only': '1rem',
      gap: '0.5rem',
      'font-size': '1rem',
    },
    filled: {
      primary: {
        background: hexFromArgb(theme.primary),
        text: hexFromArgb(theme.onPrimary),
        border: 'none',
        'state-color': hexFromArgb(theme.onPrimary),
      },
      secondary: {
        background: hexFromArgb(theme.secondary),
        text: hexFromArgb(theme.onSecondary),
        border: 'none',
        'state-color': hexFromArgb(theme.onSecondary),
      },
      tertiary: {
        background: hexFromArgb(theme.tertiary),
        text: hexFromArgb(theme.onTertiary),
        border: 'none',
        'state-color': hexFromArgb(theme.onTertiary),
      },
      danger: {
        background: hexFromArgb(theme.error),
        text: hexFromArgb(theme.onError),
        border: 'none',
        'state-color': hexFromArgb(theme.onError),
      },
    },
    tonal: {
      primary: {
        background: hexFromArgb(theme.primaryContainer),
        text: hexFromArgb(theme.onPrimaryContainer),
        border: 'none',
        'state-color': hexFromArgb(theme.onPrimaryContainer),
      },
      secondary: {
        background: hexFromArgb(theme.secondaryContainer),
        text: hexFromArgb(theme.onSecondaryContainer),
        border: 'none',
        'state-color': hexFromArgb(theme.onSecondaryContainer),
      },
      tertiary: {
        background: hexFromArgb(theme.tertiaryContainer),
        text: hexFromArgb(theme.onTertiaryContainer),
        border: 'none',
        'state-color': hexFromArgb(theme.onTertiaryContainer),
      },
      danger: {
        background: hexFromArgb(theme.error),
        text: hexFromArgb(theme.onError),
        border: 'none',
        'state-color': hexFromArgb(theme.onError),
      },
    },
    text: {
      primary: {
        background: 'transparent',
        text: hexFromArgb(theme.primary),
        border: 'none',
        'state-color': hexFromArgb(theme.primary),
      },
      secondary: {
        background: 'transparent',
        text: hexFromArgb(theme.secondary),
        border: 'none',
        'state-color': hexFromArgb(theme.secondary),
      },
      tertiary: {
        background: 'transparent',
        text: hexFromArgb(theme.tertiary),
        border: 'none',
        'state-color': hexFromArgb(theme.tertiary),
      },
      danger: {
        background: 'transparent',
        text: hexFromArgb(theme.error),
        border: 'none',
        'state-color': hexFromArgb(theme.error),
      },
    },
    outlined: {
      primary: {
        background: 'transparent',
        text: hexFromArgb(theme.primary),
        border: `1px solid ${hexFromArgb(theme.primary)}`,
        'state-color': hexFromArgb(theme.primary),
      },
      secondary: {
        background: 'transparent',
        text: hexFromArgb(theme.secondary),
        border: `1px solid ${hexFromArgb(theme.secondary)}`,
        'state-color': hexFromArgb(theme.secondary),
      },
      tertiary: {
        background: 'transparent',
        text: hexFromArgb(theme.tertiary),
        border: `1px solid ${hexFromArgb(theme.tertiary)}`,
        'state-color': hexFromArgb(theme.tertiary),
      },
      danger: {
        background: 'transparent',
        text: hexFromArgb(theme.error),
        border: `1px solid ${hexFromArgb(theme.error)}`,
        'state-color': hexFromArgb(theme.error),
      },
    },
  };
}
