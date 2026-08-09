<script setup lang="ts">
import {
  Button,
  Card,
  Select,
  Themed,
  useThemeManager,
  type DarkMode,
} from '@tak-ui-lib/components';
import type { Theme } from '@tak-ui-lib/themes';
import { blueColorScheme } from '@tak-ui-lib/themes/src/theme/schemas.ts';
import { createDefaultTheme } from '@tak-ui-lib/themes/src/theme/default.ts';
import Page from '../components/Page.vue';

const themeManager = useThemeManager();

const theme1: Theme = createDefaultTheme(blueColorScheme);

const themeOptions = [
  { label: 'Default', value: { id: 'default' } },
  { label: 'Blue', value: theme1 },
];

const darkModeOptions: { label: string; value: DarkMode }[] = [
  { label: 'Light', value: 'light' },
  { label: 'Dark', value: 'dark' },
  { label: 'System', value: 'system' },
];
</script>

<template>
  <Page>
    <h1>Theme</h1>
    <Card>
      <div class="flex flex-col gap-2">
        <Select
          label="Dark Mode"
          :model-value="themeManager.current.value.darkMode"
          :options="darkModeOptions"
          @update:model-value="themeManager.setDarkMode($event)"
        />
        <template v-for="theme in themeOptions" :key="theme.value.id">
          <Themed
            :theme="theme.value"
            :is-dark="themeManager.isDark.value"
            class="p-4 rounded-md w-full gap-2 flex flex-col"
          >
            <h2>{{ theme.label }}</h2>
            <Card>
              <Button
                severity="primary"
                label="Apply"
                @click="() => themeManager.setTheme(theme.value)"
              />
              <Button
                severity="secondary"
                label="Apply"
                @click="() => themeManager.setTheme(theme.value)"
              />
            </Card>
          </Themed>
        </template>
      </div>
    </Card>
  </Page>
</template>
