<script setup lang="ts">
import { Button, Icon } from '@tak-ui-lib/components';
import { RouterLink } from 'vue-router';

const navigationItems = [
  { label: 'Home', path: '/', icon: 'home' },
  { label: 'Components', path: '/components', icon: 'component' },
  { label: 'Themes', path: '/themes', icon: 'theme' },
];
</script>
<template>
  <div class="flex flex-col gap-2">
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
  </div>
</template>
