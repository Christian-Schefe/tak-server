<script setup lang="ts">
import { Button, Icon, useThemeManager } from '@tak-ui-lib/components';
import { RouterLink, useRoute } from 'vue-router';

defineEmits<{
  navigate: [];
}>();

type NavigationItem = {
  label: string;
  path: string;
  icon?: string;
};

const navigationItems: NavigationItem[] = [
  {
    label: 'Home',
    path: '/',
    icon: 'home',
  },
  {
    label: 'Themes',
    path: '/themes',
    icon: 'theme',
  },
];

const route = useRoute();

function isActive(path: string) {
  return route.path === path;
}

const themeManager = useThemeManager();
</script>
<template>
  <div class="grow flex flex-col gap-2">
    <template v-for="(item, index) in navigationItems" :key="index">
      <Button
        variant="text"
        :severity="isActive(item.path) ? 'primary' : 'secondary'"
        :as="{ component: RouterLink, props: { to: item.path } }"
        :label="item.label"
        @click="$emit('navigate')"
      >
        <template v-if="item.icon" #icon>
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
