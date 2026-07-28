export interface SliderSemantic {
  height: string;
  handle: SliderHandleSemantic;
  background: string;
  'hover-background': string;
  'disabled-background': string;
  'filled-background': string;
  'hover-filled-background': string;
  'disabled-filled-background': string;
}

export interface SliderHandleSemantic {
  size: string;
  background: string;
  'disabled-background': string;
  border: string;
  'focus-outline': string;
  'focus-outline-offset': string;
}
