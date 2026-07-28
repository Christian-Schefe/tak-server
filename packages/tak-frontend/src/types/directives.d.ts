import type { Directive } from 'vue';

declare module 'vue' {
  interface GlobalDirectives {
    vRipple: Directive;
  }
}

export {};
