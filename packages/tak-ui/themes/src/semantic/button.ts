export interface ButtonSemantic {
  'border-radius': string;
  size: string;
  padding: string;
  gap: string;
  'focus-outline': string;
  'focus-outline-offset': string;
  filled: ButtonSemanticVariant;
  text: ButtonSemanticVariant;
  outlined: ButtonSemanticVariant;
}

interface ButtonSemanticVariant {
  secondary: ButtonSemanticSeverity;
  primary: ButtonSemanticSeverity;
}

interface ButtonSemanticSeverity {
  background: string;
  text: string;
  'disabled-background': string;
  'disabled-text': string;
  border: string;
  hover: string;
  active: string;
}
