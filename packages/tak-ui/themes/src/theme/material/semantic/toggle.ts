import type { Switch } from '.';

export type ToggleSemantic = Switch<
  'normal',
  'hovered' | 'pressed' | 'disabled',
  Record<
    'off' | 'on',
    {
      track: {
        background: string;
        border: string;
        'border-radius': string;
      };
      handle: {
        width: string;
        height: string;
        background: string;
        'border-radius': string;
      };
    }
  > & {
    track: {
      width: string;
      height: string;
    };
    opacity: string;
  }
> & {
  focus: { outline: string; 'outline-offset': string };
};
