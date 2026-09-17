<script setup lang="ts">
import type { Theme } from '@tak-ui-lib/components';
import {
  Button,
  Card,
  Select,
  Themed,
  useThemeManager,
  type DarkMode,
} from '@tak-ui-lib/components';
import { createMaterialTheme, materialTheme } from '@tak-ui-lib/material';
import Page from '../components/Page.vue';

const themeManager = useThemeManager();

const themeOptions: { label: string; value: Theme }[] = [
  { label: 'Default', value: materialTheme },
  { label: 'Blue', value: createMaterialTheme('#448AFF') },
  { label: 'Warm', value: createMaterialTheme('#FFC107') },
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
    <div class="flex flex-col gap-2">
      <Select
        label="Dark Mode"
        :model-value="themeManager.current.value.darkMode"
        :options="darkModeOptions"
        @update:model-value="themeManager.setDarkMode($event)"
      />
      <template v-for="(theme, index) in themeOptions" :key="index">
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
  </Page>
</template>
