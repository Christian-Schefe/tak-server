import { DynamicScheme, hexFromArgb } from '@material/material-color-utilities';

export type TagSeverity = 'primary' | 'secondary' | 'tertiary' | 'danger';

export type TagTokens = {
  padding: string;
  'border-radius': string;
} & Record<
  TagSeverity,
  {
    background: string;
    text: string;
  }
>;

export function createTagTokens(theme: DynamicScheme): TagTokens {
  return {
    padding: '0.25rem 0.5rem',
    'border-radius': '0.5rem',
    primary: {
      background: hexFromArgb(theme.primaryContainer),
      text: hexFromArgb(theme.onPrimaryContainer),
    },
    secondary: {
      background: hexFromArgb(theme.secondaryContainer),
      text: hexFromArgb(theme.onSecondaryContainer),
    },
    tertiary: {
      background: hexFromArgb(theme.tertiaryContainer),
      text: hexFromArgb(theme.onTertiaryContainer),
    },
    danger: {
      background: hexFromArgb(theme.errorContainer),
      text: hexFromArgb(theme.onErrorContainer),
    },
  };
}
