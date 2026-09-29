import { hexFromArgb, type DynamicScheme } from '@material/material-color-utilities';

type CardVariants = 'elevated' | 'outlined' | 'filled';

export type CardTokens = {
  'border-radius': string;
  padding: string;
  gap: string;
} & Record<
  CardVariants,
  { background: string; text: string; border: string; 'box-shadow': string }
>;

export function createCardTokens(theme: DynamicScheme): CardTokens {
  return {
    'border-radius': '0.75rem',
    padding: '1rem',
    gap: '1rem',
    elevated: {
      background: hexFromArgb(theme.surfaceContainerLow),
      text: hexFromArgb(theme.onSurface),
      border: 'none',
      'box-shadow': `0 4px 6px ${hexFromArgb(theme.shadow)}1a`,
    },
    outlined: {
      background: hexFromArgb(theme.surface),
      text: hexFromArgb(theme.onSurface),
      border: `1px solid ${hexFromArgb(theme.outlineVariant)}`,
      'box-shadow': 'none',
    },
    filled: {
      background: hexFromArgb(theme.surfaceContainerHighest),
      text: hexFromArgb(theme.onSurface),
      border: 'none',
      'box-shadow': 'none',
    },
  };
}
