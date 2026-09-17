import { DynamicScheme, hexFromArgb } from '@material/material-color-utilities';

export type SelectTextTokens = {
  'text-empty': string;
  'text-filled': string;
  dropdown: {
    background: string;
    'border-radius': string;
    'box-shadow': string;
    padding: string;
  };
};

export function createSelectTextTokens(theme: DynamicScheme): SelectTextTokens {
  return {
    'text-empty': hexFromArgb(theme.onSurfaceVariant),
    'text-filled': hexFromArgb(theme.onSurface),
    dropdown: {
      background: hexFromArgb(theme.surface),
      'border-radius': '0.375rem',
      'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
      padding: '0.5rem',
    },
  };
}
