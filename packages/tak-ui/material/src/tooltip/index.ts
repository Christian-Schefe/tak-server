import { DynamicScheme, hexFromArgb } from '@material/material-color-utilities';

export interface TooltipTokens {
  background: string;
  text: string;
  'border-radius': string;
  'box-shadow': string;
  padding: string;
}

export function createTooltipTokens(theme: DynamicScheme): TooltipTokens {
  return {
    background: hexFromArgb(theme.inverseSurface),
    text: hexFromArgb(theme.inverseOnSurface),
    'border-radius': '0.5rem',
    'box-shadow': '0 4px 6px rgba(0, 0, 0, 0.1)',
    padding: '0.5rem',
  };
}
