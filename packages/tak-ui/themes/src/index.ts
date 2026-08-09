import type { FullTheme } from './theme';
import { defaultTheme } from './theme/default';

export type Theme = Partial<FullTheme> & { id: string };

function mergeObjects(obj1: unknown, obj2: unknown): unknown {
  if (!isObject(obj1) || !isObject(obj2)) {
    return obj2;
  }

  const result = { ...obj1 };

  for (const key of Object.keys(obj2)) {
    const value = obj2[key];

    if (value === undefined) {
      continue;
    }

    if (isObject(value) && isObject(obj1[key])) {
      result[key] = mergeObjects(obj1[key], value);
    } else {
      result[key] = value;
    }
  }

  return result;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function mergePartialTheme(partialTheme: Theme, defaultTheme: FullTheme): FullTheme {
  return mergeObjects(defaultTheme, partialTheme) as FullTheme;
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

function getVariableEntries(theme: Theme): { name: string; value: string }[] {
  const mergedTheme = mergePartialTheme(theme, defaultTheme);
  const variableEntries: { name: string; value: string }[] = [];
  traverseObject(mergedTheme.semantic, (key, value) => {
    if (typeof value === 'string') {
      const variableName = key.join('-').toLowerCase();
      variableEntries.push({ name: variableName, value });
    }
  });
  return variableEntries;
}

const cssPrefix = 'p-';

function parseLightDark(value: string, isDark: boolean): string {
  const lightDarkRegex = /\{([^}]+)\|([^}]+)\}/g;
  const result = value.replace(lightDarkRegex, (_, lightVal, darkVal) => {
    return isDark ? darkVal : lightVal;
  });
  return result;
}

export function getThemeStyles(theme: Theme, isDark: boolean): Record<string, string> {
  const variableEntries = getVariableEntries(theme);
  const styles = variableEntries.map(({ name, value }) => {
    const fullName = `--${cssPrefix}${name}`;
    return [fullName, parseLightDark(value, isDark)];
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
