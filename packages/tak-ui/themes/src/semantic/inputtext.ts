import type { Rect, Switch } from '.';

export type InputTextSemantic = Switch<
  'normal',
  'hovered' | 'focused' | 'disabled',
  {
    background: string;
    'border-radius': string;
    'text-empty': string;
    'text-filled': string;
    padding: Rect;
    width: string;
    height: string;
    outline: string;
    label: {
      top: string;
      color: string;
    };
    'icon-padding': string;
    support: {
      padding: string;
      color: string;
    };
    opacity: string;
  }
>;
