import { hexFromArgb, type DynamicScheme } from '@material/material-color-utilities';

export type BadgeTokens = {
  padding: string;
  'padding-with-label': string;
  background: string;
  text: string;
};

export function createBadgeTokens(theme: DynamicScheme): BadgeTokens {
  return {
    padding: '0.25rem',
    'padding-with-label': '0 0.375rem',
    background: hexFromArgb(theme.error),
    text: hexFromArgb(theme.onError),
  };
}
