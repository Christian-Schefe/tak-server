import type { Rect, Switch } from '.';

export type SelectSemantic = Switch<
  'normal',
  'hovered' | 'focused' | 'disabled',
  {
    background: string;
    'border-radius': string;
    'text-empty': string;
    'text-filled': string;
    padding: Rect;
    'padding-with-label': { top: string; bottom: string };
    width: string;
    height: string;
    outline: string;
    label: {
      top: string;
      color: string;
    };
    'icon-padding': string;
    'icon-color': string;
    opacity: string;
  }
> & {
  dropdown: {
    background: string;
    'border-radius': string;
    'box-shadow': string;
    padding: string;
  };
};
