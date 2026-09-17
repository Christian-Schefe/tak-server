import { hexFromArgb, type DynamicScheme } from '@material/material-color-utilities';

export interface SideBarTokens {
  background: string;
  text: string;
  border: string;
  padding: string;
  'mask-background': string;
}

export function createSideBarTokens(theme: DynamicScheme): SideBarTokens {
  return {
    background: hexFromArgb(theme.surfaceContainer),
    text: hexFromArgb(theme.onSurface),
    border: 'none',
    padding: '0.5rem',
    'mask-background': 'rgba(0, 0, 0, 0.5)',
  };
}
