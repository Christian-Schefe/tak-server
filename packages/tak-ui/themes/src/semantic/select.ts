import type { Vector2 } from '.';

export interface SelectSemantic {
  background: string;
  'border-radius': string;
  border: string;
  'border-hover': string;
  'border-focus': string;
  padding: Vector2;
  dropdown: SelectDropdownSemantic;
  'filled-text': string;
  'empty-text': string;
  'icon-color': string;
  'label-text-focus': string;
}

interface SelectDropdownSemantic {
  background: string;
  'border-radius': string;
  'box-shadow': string;
  padding: string;
}
