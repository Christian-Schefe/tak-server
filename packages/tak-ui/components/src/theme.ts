import { applyTheme, type Theme } from '@tak-ui-lib/themes';
import { useMediaQuery } from '@vueuse/core';
import { computed, inject, ref, watch, type App, type InjectionKey, type Ref } from 'vue';

export type DarkMode = 'dark' | 'light' | 'system';

export class ThemeManager {
  current = ref<{ theme: Theme; darkMode: DarkMode }>({
    theme: { id: 'default' },
    darkMode: 'system',
  });
  isDark: Ref<boolean>;

  constructor() {
    const systemPrefersDarkMode = useMediaQuery('(prefers-color-scheme: dark)');
    this.isDark = computed(() => {
      const darkMode = this.current.value.darkMode;
      return darkMode === 'dark' || (darkMode === 'system' && systemPrefersDarkMode.value);
    });
    watch(
      [() => this.current.value.theme, this.isDark],
      ([newTheme, newIsDark]) => {
        applyTheme(newTheme, newIsDark);
      },
      { immediate: true },
    );
  }

  toggleDarkMode(): void {
    this.setDarkMode(this.isDark.value ? 'light' : 'dark');
  }

  setDarkMode(darkMode: DarkMode): void {
    this.current.value.darkMode = darkMode;
  }

  setTheme(theme: Theme): void {
    this.current.value.theme = theme;
  }

  setThemeAndDarkMode(theme: Theme, darkMode: DarkMode): void {
    this.current.value = { theme, darkMode };
  }
}

const ThemeKey: InjectionKey<ThemeManager> = Symbol('ThemeManager');

export function provideThemeManager(app: App) {
  const themeManager = new ThemeManager();
  app.provide(ThemeKey, themeManager);
}

export function useThemeManager(): ThemeManager {
  const themeManager = inject(ThemeKey);
  if (!themeManager) {
    throw new Error('ThemeManager not found');
  }
  return themeManager;
}
