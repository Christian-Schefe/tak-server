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
    'padding-with-label': { top: string; bottom: string };
    width: string;
    height: string;
    border: string;
    outline: string;
    label: {
      top: string;
      color: string;
    };
    icon: {
      padding: string;
      color: string;
    };
    support: {
      padding: string;
      color: string;
    };
    opacity: string;
  }
>;
