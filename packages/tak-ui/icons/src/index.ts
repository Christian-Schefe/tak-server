import type { Component } from 'vue';
import { LuChevronDown, LuMenu, LuX } from 'vue-icons-plus/lu';

export type Icon = Component | string;

export interface IconRegistry {
  [key: string]: Icon | undefined;
}

const defaultIcons: IconRegistry = {
  close: LuX,
  'chevron-down': LuChevronDown,
  menu: LuMenu,
};

export function getIconFromRegistry(registry: IconRegistry, name: string): Icon | undefined {
  return registry[name] ?? defaultIcons[name];
}
