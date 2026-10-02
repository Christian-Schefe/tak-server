import { DynamicScheme, hexFromArgb } from '@material/material-color-utilities';

export type SelectTokens = {
  'text-empty': string;
  'text-filled': string;
  dropdown: {
    'min-width': string;
    background: string;
    'border-radius': string;
    'box-shadow': string;
    padding: string;
    'button-border-radius': string;
    'button-border-radius-end': string;
  };
};

export function createSelectTokens(theme: DynamicScheme): SelectTokens {
  return {
    'text-empty': hexFromArgb(theme.onSurfaceVariant),
    'text-filled': hexFromArgb(theme.onSurface),
    dropdown: {
      'min-width': '10rem',
      background: hexFromArgb(theme.surfaceContainerLow),
      'border-radius': '1rem',
      'box-shadow': `0 4px 6px ${hexFromArgb(theme.shadow)}40`,
      padding: '0.25rem',
      'button-border-radius': '0.5rem',
      'button-border-radius-end': '1rem',
    },
  };
}
