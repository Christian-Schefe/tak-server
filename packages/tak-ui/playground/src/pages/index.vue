<script setup lang="ts">
import { Button } from '@tak-ui-lib/components';
import Page from '../components/Page.vue';
import { RouterLink } from 'vue-router';
const components = [
  {
    label: 'Button',
    link: '/components/button',
  },
  {
    label: 'Input',
    link: '/components/input',
  },
  {
    label: 'Select',
    link: '/components/select',
  },
  {
    label: 'Tooltip',
    link: '/components/tooltip',
  },
  {
    label: 'Dialog',
    link: '/components/dialog',
  },
  {
    label: 'Slider',
    link: '/components/slider',
  },
  { label: 'Toggle', link: '/components/toggle' },
  { label: 'Card', link: '/components/card' },
];
</script>
<template>
  <Page>
    <h1>Tak UI</h1>
    <h2>Components</h2>
    <div class="flex flex-col gap-2">
      <template v-for="component in components" :key="component.label">
        <Button
          :as="{ component: RouterLink, props: { to: component.link } }"
          severity="secondary"
          :label="component.label"
        >
        </Button>
      </template>
    </div>
  </Page>
</template>
