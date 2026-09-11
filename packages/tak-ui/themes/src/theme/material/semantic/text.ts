export interface TextSemantic {
  small: TextSemanticSize;
  medium: TextSemanticSize;
  large: TextSemanticSize;
}

interface TextSemanticSize {
  size: string;
  'line-height': string;
}
