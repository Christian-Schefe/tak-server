export interface DropdownTokens {
  'transform-enter-from': string;
}

export function createDropdownTokens(): DropdownTokens {
  return {
    'transform-enter-from': 'scale(0.9)',
  };
}
