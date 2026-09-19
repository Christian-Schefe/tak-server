import { DynamicScheme, hexFromArgb } from '@material/material-color-utilities';

export type LabelFieldTokens = {
  background: string;
  'hovered-background': string;
  'focused-background': string;
  'border-radius': string;
  'text-empty': string;
  'text-filled': string;
  padding: {
    top: string;
    right: string;
    bottom: string;
    left: string;
  };
  'padding-with-label': { top: string; bottom: string };
  height: string;
  outline: string;
  'focus-outline': string;
  label: {
    top: string;
    color: string;
    'focus-color': string;
  };
  icon: {
    padding: string;
    color: string;
    'focus-color': string;
  };
  support: {
    padding: string;
    color: string;
  };
  'disabled-opacity': string;
};

export function createLabelFieldTokens(theme: DynamicScheme): LabelFieldTokens {
  return {
    background: hexFromArgb(theme.surfaceContainerHigh),
    'hovered-background': hexFromArgb(theme.surfaceContainerHighest),
    'focused-background': hexFromArgb(theme.surfaceContainerHighest),
    'border-radius': '0.375rem',
    'text-empty': hexFromArgb(theme.onSurfaceVariant),
    'text-filled': hexFromArgb(theme.onSurface),
    padding: { left: '0.75rem', top: '0.25rem', right: '0.75rem', bottom: '0.25rem' },
    'padding-with-label': { top: '1rem', bottom: '0.25rem' },
    height: '3rem',
    outline: `2px solid transparent`,
    'focus-outline': `2px solid ${hexFromArgb(theme.primary)}`,
    label: {
      top: '0.375rem',
      color: hexFromArgb(theme.onSurfaceVariant),
      'focus-color': hexFromArgb(theme.primary),
    },
    icon: {
      padding: '0.75rem',
      color: hexFromArgb(theme.onSurfaceVariant),
      'focus-color': hexFromArgb(theme.primary),
    },
    support: {
      padding: '0.25rem 0.75rem',
      color: hexFromArgb(theme.onSurfaceVariant),
    },
    'disabled-opacity': '0.5',
  };
}
