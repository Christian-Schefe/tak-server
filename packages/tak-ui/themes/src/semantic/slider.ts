import type { Switch } from '.';

export type SliderSemantic = Switch<
  'normal',
  'hovered' | 'pressed' | 'disabled',
  {
    track: {
      padding: string;
      gap: string;
      height: string;
      'border-radius': string;
    } & Record<'unfilled' | 'filled', { background: string }>;
    handle: {
      width: string;
      height: string;
      background: string;
      'border-radius': string;
    };
    opacity: string;
  }
> & { focus: { outline: string; 'outline-offset': string } };
