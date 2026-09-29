import { hexFromArgb, type DynamicScheme } from '@material/material-color-utilities';

export interface DialogTokens {
  'border-radius': string;
  'box-shadow': string;
  background: string;
  text: string;
  padding: string;
  'mask-background': string;
  'font-size': string;
  'line-height': string;
}

export function createDialogTokens(theme: DynamicScheme): DialogTokens {
  return {
    'border-radius': '1rem',
    'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
    background: hexFromArgb(theme.surfaceContainerLow),
    text: hexFromArgb(theme.onSurface),
    padding: '1rem',
    'mask-background': `${hexFromArgb(theme.scrim)}80`,
    'font-size': '1.25rem',
    'line-height': '2rem',
  };
}
