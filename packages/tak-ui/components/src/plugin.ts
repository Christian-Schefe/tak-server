import { type App, type EffectScope, effectScope } from 'vue';
import { provideThemeManager, type DarkMode, type Theme } from './theme';
import { provideIconManager } from './icons';
import { provideOverlayManager } from './overlay';
import type { Icon } from '@tak-ui-lib/icons';

type PluginOptions = {
  icons?: Record<string, Icon>;
  theme?: Theme;
  darkMode?: DarkMode;
};

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
        provideThemeManager(
          app,
          options?.theme ?? { light: {}, dark: {} },
          options?.darkMode ?? 'system',
        );
        provideIconManager(app, options?.icons);
        provideOverlayManager(app);
      });
    },
  };
}

export function disposeTakUI(plugin: TakUI) {
  plugin.effectScope.stop();
}
