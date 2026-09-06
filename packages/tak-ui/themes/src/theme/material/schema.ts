import {
  argbFromHex,
  DynamicScheme,
  Hct,
  hexFromArgb,
  Variant,
} from '@material/material-color-utilities';

export type ColorType = 'primary' | 'secondary' | 'neutral';

export type ColorScheme = {
  primary: string;
  onPrimary: string;

  primaryContainer: string;
  onPrimaryContainer: string;

  secondary: string;
  onSecondary: string;

  secondaryContainer: string;
  onSecondaryContainer: string;

  surface: string;
  onSurface: string;

  surfaceVariant: string;
  onSurfaceVariant: string;

  surfaceContainerLowest: string;
  surfaceContainerLow: string;
  surfaceContainer: string;
  surfaceContainerHigh: string;
  surfaceContainerHighest: string;

  inverseSurface: string;
  onInverseSurface: string;

  outline: string;
  outlineVariant: string;
};

export function createMaterialColorScheme(sourceColor: string, isDark: boolean): ColorScheme {
  const themeColor = Hct.fromInt(argbFromHex(sourceColor));
  const theme = new DynamicScheme({
    sourceColorHct: themeColor,
    variant: Variant.TONAL_SPOT,
    contrastLevel: 0,
    isDark,
  });
  return {
    primary: hexFromArgb(theme.primary),
    onPrimary: hexFromArgb(theme.onPrimary),

    primaryContainer: hexFromArgb(theme.primaryContainer),
    onPrimaryContainer: hexFromArgb(theme.onPrimaryContainer),

    secondary: hexFromArgb(theme.secondary),
    onSecondary: hexFromArgb(theme.onSecondary),

    secondaryContainer: hexFromArgb(theme.secondaryContainer),
    onSecondaryContainer: hexFromArgb(theme.onSecondaryContainer),

    surface: hexFromArgb(theme.surface),
    onSurface: hexFromArgb(theme.onSurface),

    surfaceVariant: hexFromArgb(theme.surfaceVariant),
    onSurfaceVariant: hexFromArgb(theme.onSurfaceVariant),

    surfaceContainerLowest: hexFromArgb(theme.surfaceDim),
    surfaceContainerLow: hexFromArgb(theme.surfaceBright),
    surfaceContainer: hexFromArgb(theme.surfaceContainer),
    surfaceContainerHigh: hexFromArgb(theme.surfaceContainerHigh),
    surfaceContainerHighest: hexFromArgb(theme.surfaceContainerHighest),

    inverseSurface: hexFromArgb(theme.inverseSurface),
    onInverseSurface: hexFromArgb(theme.inverseOnSurface),

    outline: hexFromArgb(theme.outline),
    outlineVariant: hexFromArgb(theme.outlineVariant),
  };
}
