<script setup lang="ts">
import { Button, Icon, useThemeManager } from '@tak-ui-lib/components';
import { RouterLink } from 'vue-router';

const navigationItems = [
  { label: 'Home', path: '/', icon: 'home' },
  { label: 'Components', path: '/components', icon: 'component' },
  { label: 'Forms', path: '/components/form', icon: 'component' },
  { label: 'Themes', path: '/themes', icon: 'theme' },
];

const themeManager = useThemeManager();
</script>
<template>
  <div class="grow flex flex-col gap-2">
    <template v-for="item in navigationItems" :key="item.path">
      <Button
        variant="text"
        severity="secondary"
        :as="{ component: RouterLink, props: { to: item.path } }"
        :label="item.label"
      >
        <template #icon>
          <Icon :name="item.icon" />
        </template>
      </Button>
    </template>
    <div class="grow"></div>
    <div class="flex">
      <Button variant="text" severity="secondary" icon-only @click="themeManager.toggleDarkMode()">
        <Icon :name="themeManager.isDark.value ? 'darkMode' : 'lightMode'" />
      </Button>
    </div>
  </div>
</template>
