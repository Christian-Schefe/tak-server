import { hexFromArgb, type DynamicScheme } from '@material/material-color-utilities';

export type ButtonVariant = 'filled' | 'text' | 'outlined';
export type ButtonSeverity = 'primary' | 'secondary';

export type ButtonTokens = {
  'border-radius': string;
  padding: string;
  gap: string;
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
>;

export function createButtonTokens(theme: DynamicScheme): ButtonTokens {
  return {
    'border-radius': '0.375rem',
    padding: '0.5rem',
    gap: '0.5rem',
    'hover-state-opacity': '0.1',
    'pressed-state-opacity': '0.2',
    'disabled-opacity': '0.5',
    'focus-outline': `2px solid ${hexFromArgb(theme.primary)}`,
    'focus-outline-offset': '2px',
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
    },
  };
}
