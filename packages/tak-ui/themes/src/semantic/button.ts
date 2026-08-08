import type { Switch } from '.';

interface ButtonSemanticVariant {
  secondary: ButtonSemanticSeverity;
  primary: ButtonSemanticSeverity;
}

interface ButtonSemanticSeverity {
  background: string;
  text: string;
  border: string;
}

export type ButtonSemantic = Switch<
  'normal',
  'hovered' | 'pressed' | 'disabled',
  {
    'border-radius': string;
    size: string;
    padding: string;
    gap: string;
    opacity: string;
    filled: ButtonSemanticVariant;
    text: ButtonSemanticVariant;
    outlined: ButtonSemanticVariant;
  }
> & { focus: { outline: string; 'outline-offset': string } };
