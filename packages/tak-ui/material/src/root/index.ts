import { hexFromArgb, type DynamicScheme } from '@material/material-color-utilities';

export interface RootTokens {
  background: string;
  text: string;
  'text-size': string;
  'text-line-height': string;
  'scrollbar-thumb': string;
  'scrollbar-track': string;
}

export function createRootTokens(theme: DynamicScheme): RootTokens {
  return {
    background: hexFromArgb(theme.surface),
    text: hexFromArgb(theme.onSurface),
    'text-size': '1rem',
    'text-line-height': 'calc(1.5rem / 1rem)',
    'scrollbar-track': hexFromArgb(theme.surface),
    'scrollbar-thumb': hexFromArgb(theme.outline),
  };
}
