export type ColorScheme = {
  primary: string;
  primaryHover: string;
  primaryActive: string;
  onPrimary: string;

  primaryContainer: string;
  primaryContainerHover: string;
  onPrimaryContainer: string;

  secondary: string;
  secondaryHover: string;
  onSecondary: string;

  secondaryContainer: string;
  secondaryContainerHover: string;
  secondaryContainerActive: string;
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
  onSurfaceContainer: string;

  inverseSurface: string;
  onInverseSurface: string;

  outline: string;
  outlineVariant: string;
};

function lightDarkColorScheme(light: ColorScheme, dark: ColorScheme): ColorScheme {
  function lightDark(light: string, dark: string): string {
    return `{${light}|${dark}}`;
  }
  return {
    primary: lightDark(light.primary, dark.primary),
    primaryHover: lightDark(light.primaryHover, dark.primaryHover),
    primaryActive: lightDark(light.primaryActive, dark.primaryActive),
    onPrimary: lightDark(light.onPrimary, dark.onPrimary),

    primaryContainer: lightDark(light.primaryContainer, dark.primaryContainer),
    primaryContainerHover: lightDark(light.primaryContainerHover, dark.primaryContainerHover),
    onPrimaryContainer: lightDark(light.onPrimaryContainer, dark.onPrimaryContainer),

    secondary: lightDark(light.secondary, dark.secondary),
    secondaryHover: lightDark(light.secondaryHover, dark.secondaryHover),
    onSecondary: lightDark(light.onSecondary, dark.onSecondary),

    secondaryContainer: lightDark(light.secondaryContainer, dark.secondaryContainer),
    secondaryContainerHover: lightDark(light.secondaryContainerHover, dark.secondaryContainerHover),
    secondaryContainerActive: lightDark(
      light.secondaryContainerActive,
      dark.secondaryContainerActive,
    ),
    onSecondaryContainer: lightDark(light.onSecondaryContainer, dark.onSecondaryContainer),

    surface: lightDark(light.surface, dark.surface),
    onSurface: lightDark(light.onSurface, dark.onSurface),
    surfaceVariant: lightDark(light.surfaceVariant, dark.surfaceVariant),
    onSurfaceVariant: lightDark(light.onSurfaceVariant, dark.onSurfaceVariant),

    surfaceContainerLowest: lightDark(light.surfaceContainerLowest, dark.surfaceContainerLowest),
    surfaceContainerLow: lightDark(light.surfaceContainerLow, dark.surfaceContainerLow),
    surfaceContainer: lightDark(light.surfaceContainer, dark.surfaceContainer),
    surfaceContainerHigh: lightDark(light.surfaceContainerHigh, dark.surfaceContainerHigh),
    surfaceContainerHighest: lightDark(light.surfaceContainerHighest, dark.surfaceContainerHighest),
    onSurfaceContainer: lightDark(light.onSurfaceContainer, dark.onSurfaceContainer),

    inverseSurface: lightDark(light.inverseSurface, dark.inverseSurface),
    onInverseSurface: lightDark(light.onInverseSurface, dark.onInverseSurface),

    outline: lightDark(light.outline, dark.outline),
    outlineVariant: lightDark(light.outlineVariant, dark.outlineVariant),
  };
}

const materialLightCS: ColorScheme = {
  primary: '#6750A4',
  primaryHover: '#4b3b78',
  primaryActive: '#3a2c5e',
  onPrimary: '#FFFFFF',

  primaryContainer: '#EADDFF',
  primaryContainerHover: '#D0BCFF',
  onPrimaryContainer: '#4F378B',

  secondary: '#625B71',
  secondaryHover: '#7A7287',
  onSecondary: '#FFFFFF',

  secondaryContainer: '#E8DEF8',
  secondaryContainerHover: '#CCC2DC',
  secondaryContainerActive: '#B0A6C0',
  onSecondaryContainer: '#4A4458',

  surface: '#FEF7FF',
  onSurface: '#1D1B20',
  surfaceVariant: '#E7E0EC',
  onSurfaceVariant: '#49454F',

  surfaceContainerLowest: '#FFFFFF',
  surfaceContainerLow: '#F7F2FA',
  surfaceContainer: '#F3EDF7',
  surfaceContainerHigh: '#ECE6F0',
  surfaceContainerHighest: '#E6E0E9',
  onSurfaceContainer: '#1D1B20',

  inverseSurface: '#322F35',
  onInverseSurface: '#F5EFF7',

  outline: '#79747E',
  outlineVariant: '#CAC4D0',
};
const materialDarkCS: ColorScheme = {
  primary: '#D0BCFF',
  primaryHover: '#EADDFF',
  primaryActive: '#FFEEFF',
  onPrimary: '#381E72',

  primaryContainer: '#4F378B',
  primaryContainerHover: '#6750A4',
  onPrimaryContainer: '#EADDFF',

  secondary: '#CCC2DC',
  secondaryHover: '#E8DEF8',
  onSecondary: '#332D41',

  secondaryContainer: '#4A4458',
  secondaryContainerHover: '#5F576B',
  secondaryContainerActive: '#6B6178',
  onSecondaryContainer: '#E8DEF8',

  surface: '#141218',
  onSurface: '#E6E0E9',
  surfaceVariant: '#49454F',
  onSurfaceVariant: '#CAC4D0',

  surfaceContainerLowest: '#0F0D13',
  surfaceContainerLow: '#1D1B20',
  surfaceContainer: '#211F26',
  surfaceContainerHigh: '#2B2930',
  surfaceContainerHighest: '#36343B',
  onSurfaceContainer: '#E6E0E9',

  inverseSurface: '#E6E0E9',
  onInverseSurface: '#322F35',

  outline: '#938F99',
  outlineVariant: '#49454F',
};

export const materialColorScheme = lightDarkColorScheme(materialLightCS, materialDarkCS);
