import { hexFromArgb, type DynamicScheme } from '@material/material-color-utilities';

export interface DialogTokens {
  'border-radius': string;
  'box-shadow': string;
  background: string;
  padding: string;
  'mask-background': string;
}

export function createDialogTokens(theme: DynamicScheme): DialogTokens {
  return {
    'border-radius': '0.5rem',
    'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
    background: hexFromArgb(theme.surface),
    padding: '1rem',
    'mask-background': 'rgba(0, 0, 0, 0.5)',
  };
}
