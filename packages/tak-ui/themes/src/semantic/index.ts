import type { ButtonSemantic } from './button';
import type { CardSemantic } from './card';
import type { ColorsSemantic } from './colors';
import type { DialogSemantic } from './dialog';
import type { InputTextSemantic } from './inputtext';
import type { ScrollbarSemantic } from './scrollbar';
import type { SelectSemantic } from './select';
import type { SideBarSemantic } from './sidebar';
import type { SliderSemantic } from './slider';
import type { TextSemantic } from './text';
import type { ToggleSemantic } from './toggle';
import type { TooltipSemantic } from './tooltip';

export interface ThemeSemantic {
  color: ColorsSemantic;
  button: ButtonSemantic;
  inputtext: InputTextSemantic;
  card: CardSemantic;
  dialog: DialogSemantic;
  text: TextSemantic;
  scrollbar: ScrollbarSemantic;
  slider: SliderSemantic;
  select: SelectSemantic;
  sidebar: SideBarSemantic;
  tooltip: TooltipSemantic;
  toggle: ToggleSemantic;
}

export interface Vector2 {
  x: string;
  y: string;
}
