<script setup lang="ts">
import { App, Button, Icon, SideBar } from '@tak-ui-lib/components';
import { breakpointsTailwind, useBreakpoints } from '@vueuse/core';
import { ref, watch } from 'vue';
import Navigation from './components/Navigation.vue';

const sidebarVisible = ref(false);

const breakpoints = useBreakpoints(breakpointsTailwind);
const isMobile = breakpoints.smaller('md');

watch(isMobile, (newIsMobile) => {
  if (newIsMobile) {
    sidebarVisible.value = false;
  }
});
</script>

<template>
  <App>
    <template #top>
      <SideBar :visible="isMobile" direction="top">
        <Button
          variant="text"
          severity="secondary"
          icon-only
          @click="sidebarVisible = !sidebarVisible"
        >
          <Icon :name="sidebarVisible ? 'close' : 'menu'" />
        </Button>
      </SideBar>
    </template>
    <template #left>
      <SideBar
        :visible="sidebarVisible || !isMobile"
        direction="left"
        :overlay="isMobile"
        @update:visible="sidebarVisible = $event"
      >
        <Navigation @navigate="sidebarVisible = false" />
      </SideBar>
    </template>
    <RouterView />
  </App>
</template>
