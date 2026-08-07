import type { Switch } from '.';

export type ToggleSemantic = Record<
  'off' | 'on',
  Switch<
    'normal',
    'hovered' | 'pressed' | 'disabled',
    {
      track: {
        width: string;
        height: string;
        background: string;
        'border-radius': string;
      };
      handle: {
        width: string;
        height: string;
        background: string;
        'border-radius': string;
      };
    }
  >
> & { focus: { outline: string; 'outline-offset': string } };
