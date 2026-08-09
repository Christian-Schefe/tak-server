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
  onSurfaceVariant: string;

  surfaceContainer: string;
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
    onSurfaceVariant: lightDark(light.onSurfaceVariant, dark.onSurfaceVariant),

    surfaceContainer: lightDark(light.surfaceContainer, dark.surfaceContainer),
    onSurfaceContainer: lightDark(light.onSurfaceContainer, dark.onSurfaceContainer),

    inverseSurface: lightDark(light.inverseSurface, dark.inverseSurface),
    onInverseSurface: lightDark(light.onInverseSurface, dark.onInverseSurface),

    outline: lightDark(light.outline, dark.outline),
    outlineVariant: lightDark(light.outlineVariant, dark.outlineVariant),
  };
}

const blueLightCS: ColorScheme = {
  primary: '#2563EB',
  primaryHover: '#1D4ED8',
  primaryActive: '#133C9A',
  onPrimary: '#FFFFFF',
  primaryContainer: '#DBEAFE',
  primaryContainerHover: '#BFDBFE',
  onPrimaryContainer: '#1E3A8A',

  secondary: '#5F6B7A',
  secondaryHover: '#4B5563',
  onSecondary: '#FFFFFF',
  secondaryContainer: '#E5EAF0',
  secondaryContainerHover: '#D6DCE4',
  secondaryContainerActive: '#C0C9D4',
  onSecondaryContainer: '#344054',

  surface: '#F8FAFC',
  onSurface: '#172033',
  onSurfaceVariant: '#596579',
  surfaceContainer: '#EEF2F6',
  onSurfaceContainer: '#172033',

  inverseSurface: '#202938',
  onInverseSurface: '#F3F6FA',

  outline: '#7B8798',
  outlineVariant: '#CBD2DC',
};

const blueDarkCS: ColorScheme = {
  primary: '#7AA7FF',
  primaryHover: '#A8C4FF',
  primaryActive: '#C0D9FF',
  onPrimary: '#082E81',
  primaryContainer: '#1746A2',
  primaryContainerHover: '#1D4ED8',
  onPrimaryContainer: '#DBEAFE',

  secondary: '#B8C1CE',
  secondaryHover: '#D0D7E0',
  onSecondary: '#27313F',
  secondaryContainer: '#3B4655',
  secondaryContainerHover: '#4A5666',
  secondaryContainerActive: '#5A677C',
  onSecondaryContainer: '#E5EAF0',

  surface: '#11161D',
  onSurface: '#E8EDF3',
  onSurfaceVariant: '#B1BAC7',
  surfaceContainer: '#1B222C',
  onSurfaceContainer: '#E8EDF3',

  inverseSurface: '#E8EDF3',
  onInverseSurface: '#202938',

  outline: '#7E8998',
  outlineVariant: '#424B58',
};
export const blueColorScheme = lightDarkColorScheme(blueLightCS, blueDarkCS);

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
  onSurfaceVariant: '#49454F',

  surfaceContainer: '#F3EDF7',
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
  onSurfaceVariant: '#CAC4D0',

  surfaceContainer: '#211F26',
  onSurfaceContainer: '#E6E0E9',

  inverseSurface: '#E6E0E9',
  onInverseSurface: '#322F35',

  outline: '#938F99',
  outlineVariant: '#49454F',
};

export const materialColorScheme = lightDarkColorScheme(materialLightCS, materialDarkCS);
