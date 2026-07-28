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
import { gray, green, rose, slate, stone, viva } from '@tak-ui-lib/themes/src/theme/colors.ts';
import Page from '../components/Page.vue';

const themeManager = useThemeManager();

const theme1: Theme = {
  id: 'theme1',
  colors: {
    primary: gray,
    surface: stone,
  },
};
const theme2: Theme = {
  id: 'theme2',
  colors: {
    primary: rose,
    surface: viva,
  },
};
const theme3: Theme = {
  id: 'theme3',
  colors: {
    primary: green,
    surface: slate,
  },
};

const themeOptions = [
  { label: 'Default', value: { id: 'default' } },
  { label: 'Monochrome', value: theme1 },
  { label: 'Rose', value: theme2 },
  { label: 'Mint', value: theme3 },
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
