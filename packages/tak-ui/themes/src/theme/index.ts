import type { PrimaryColor, SurfaceColor } from './colors';
import type { ThemeSemantic } from '../semantic';

export interface FullTheme {
  id: string;
  colors: {
    primary: PrimaryColor;
    surface: SurfaceColor;
  };
  semantic: ThemeSemantic;
}
