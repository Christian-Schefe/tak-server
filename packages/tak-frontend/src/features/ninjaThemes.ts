export const ninja2dThemes = [
  'aaron',
  'aer',
  'aether',
  'aqua',
  'atlas',
  'backlit',
  'bubbletron',
  'classic',
  'discord',
  'essence',
  'fresh',
  'ignis',
  'luna',
  'paper',
  'retro',
  'stealth',
  'terra',
  'zen',
] as const;

export type Ninja2DTheme = (typeof ninja2dThemes)[number];
