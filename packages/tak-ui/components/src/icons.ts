import { getIconFromRegistry, type Icon, type IconRegistry } from '@tak-ui-lib/icons';
import {
  computed,
  inject,
  shallowRef,
  toValue,
  type App,
  type InjectionKey,
  type MaybeRefOrGetter,
} from 'vue';

export class IconManager {
  private iconRegistry = shallowRef<IconRegistry>({});

  registerIcons(icons: Record<string, Icon>): void {
    this.iconRegistry.value = { ...this.iconRegistry.value, ...icons };
  }

  getIcon(name: string): Icon | undefined {
    return getIconFromRegistry(this.iconRegistry.value, name);
  }
}

const IconKey: InjectionKey<IconManager> = Symbol('IconManager');

export function provideIconManager(app: App, icons?: Record<string, Icon>) {
  const iconManager = new IconManager();
  if (icons) {
    iconManager.registerIcons(icons);
  }
  app.provide(IconKey, iconManager);
}

export function useIconManager(): IconManager {
  const iconManager = inject(IconKey);
  if (!iconManager) {
    throw new Error('IconManager not found');
  }
  return iconManager;
}

export function useIcon(name: MaybeRefOrGetter<string>) {
  const iconManager = useIconManager();
  return computed(() => {
    return iconManager.getIcon(toValue(name));
  });
}
