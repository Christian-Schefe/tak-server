import type { FullTheme } from './theme';
export { materialTheme } from './theme/material';

export type Theme = {
  light: FullTheme;
  dark: FullTheme;
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
  traverseObject(activeTheme.semantic, (key, value) => {
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
    console.log(`Set CSS variable ${name} to ${value}`);
  });
  document.documentElement.dataset['theme'] = isDark ? 'dark' : 'light';
}
