import { hexFromArgb, type DynamicScheme } from '@material/material-color-utilities';

export type BadgeTokens = {
  'border-radius': string;
} & Record<'primary' | 'error', { background: string; text: string }>;

export function createBadgeTokens(theme: DynamicScheme): BadgeTokens {
  return {
    'border-radius': '4px',
    primary: {
      background: hexFromArgb(theme.primary),
      text: hexFromArgb(theme.onPrimary),
    },
    error: {
      background: hexFromArgb(theme.error),
      text: hexFromArgb(theme.onError),
    },
  };
}
