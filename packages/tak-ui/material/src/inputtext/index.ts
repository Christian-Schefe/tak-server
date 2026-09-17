import { DynamicScheme, hexFromArgb } from '@material/material-color-utilities';

export type InputTextTokens = {
  'text-empty': string;
  'text-filled': string;
};

export function createInputTextTokens(theme: DynamicScheme): InputTextTokens {
  return {
    'text-empty': hexFromArgb(theme.onSurfaceVariant),
    'text-filled': hexFromArgb(theme.onSurface),
  };
}
