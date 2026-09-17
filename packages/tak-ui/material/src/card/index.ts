import { hexFromArgb, type DynamicScheme } from '@material/material-color-utilities';

export interface CardTokens {
  'border-radius': string;
  padding: string;
  gap: string;
  background: string;
  text: string;
  border: string;
  'box-shadow': string;
}

export function createCardTokens(theme: DynamicScheme): CardTokens {
  return {
    'border-radius': '0.5rem',
    padding: '1rem',
    gap: '1rem',
    background: hexFromArgb(theme.surfaceContainerLow),
    text: hexFromArgb(theme.onSurface),
    border: `none`,
    'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
  };
}
