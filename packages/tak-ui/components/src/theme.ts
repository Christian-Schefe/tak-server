import { useMediaQuery } from '@vueuse/core';
import { computed, inject, ref, watch, type App, type InjectionKey, type Ref } from 'vue';
export type DarkMode = 'dark' | 'light' | 'system';

export type Theme = {
  light: unknown;
  dark: unknown;
};

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function traverseObject(
  obj: unknown,
  callback: (key: string[], value: unknown) => void,
  prefix: string[] = [],
): void {
  if (!isObject(obj)) {
    callback(prefix, obj);
    return;
  }
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = [...prefix, key];
    traverseObject(value, callback, fullKey);
  }
}

function getVariableEntries(theme: Theme, isDark: boolean): { name: string; value: string }[] {
  const activeTheme = isDark ? theme.dark : theme.light;
  const variableEntries: { name: string; value: string }[] = [];
  traverseObject(activeTheme, (key, value) => {
    if (typeof value === 'string') {
      const variableName = key.join('-').toLowerCase();
      variableEntries.push({ name: variableName, value });
    }
  });
  return variableEntries;
}

const cssPrefix = 'p-';

export function getThemeStyles(theme: Theme, isDark: boolean): Record<string, string> {
  const variableEntries = getVariableEntries(theme, isDark);
  const styles = variableEntries.map(({ name, value }) => {
    return [`--${cssPrefix}${name}`, value];
  });
  return Object.fromEntries(styles);
}

export function applyTheme(theme: Theme, isDark: boolean): void {
  const styles = getThemeStyles(theme, isDark);
  Object.entries(styles).forEach(([name, value]) => {
    document.documentElement.style.setProperty(name, value);
  });
  document.documentElement.dataset['theme'] = isDark ? 'dark' : 'light';
}

export class ThemeManager {
  current: Ref<{ theme: Theme; darkMode: DarkMode }>;
  isDark: Ref<boolean>;

  constructor(theme: Theme, darkMode: DarkMode = 'system') {
    this.current = ref({ theme, darkMode });
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

export function provideThemeManager(app: App, theme: Theme, darkMode: DarkMode = 'system') {
  const themeManager = new ThemeManager(theme, darkMode);
  app.provide(ThemeKey, themeManager);
}

export function useThemeManager(): ThemeManager {
  const themeManager = inject(ThemeKey);
  if (!themeManager) {
    throw new Error('ThemeManager not found');
  }
  return themeManager;
}
