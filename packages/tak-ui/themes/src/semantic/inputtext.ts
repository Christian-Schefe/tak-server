import type { Switch, Vector2 } from '.';

export type InputTextSemantic = Switch<
  'normal',
  'hovered' | 'focused' | 'disabled',
  {
    background: string;
    'border-radius': string;
    'text-empty': string;
    'text-filled': string;
    padding: Vector2;
    width: string;
    outline: string;
    label: {
      padding: string;
      color: string;
    };
    support: {
      padding: string;
      color: string;
    };
  }
>;
