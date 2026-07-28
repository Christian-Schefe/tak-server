import { type App, type EffectScope, effectScope } from 'vue';
import { provideThemeManager } from './theme';
import { provideIconManager, type IconOptions } from './icons';
import { provideOverlayManager } from './overlay';

type PluginOptions = IconOptions;

export interface TakUI {
  effectScope: EffectScope;
  install(app: App, options?: PluginOptions): void;
}

export function createTakUI(): TakUI {
  const scope = effectScope(true);
  return {
    effectScope: scope,
    install(app: App, options?: PluginOptions) {
      scope.run(() => {
        provideThemeManager(app);
        provideIconManager(app, options);
        provideOverlayManager(app);
      });
    },
  };
}

export function disposeTakUI(plugin: TakUI) {
  plugin.effectScope.stop();
}
