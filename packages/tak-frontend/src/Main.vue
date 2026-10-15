<script setup lang="ts">
import { AppFrame, Button, Icon, SideBar } from '@tak-ui-lib/components';
import { breakpointsTailwind, useBreakpoints } from '@vueuse/core';
import { computed, ref, watch } from 'vue';
import { useNavigateOnGamesStartWebSocketSubscription } from './api/game.ts';
import Navigation from './components/Navigation.vue';
import { useWebSocketStore } from './features/websocket.ts';
import { useRoute } from 'vue-router';

useWebSocketStore();
useNavigateOnGamesStartWebSocketSubscription();

const sidebarVisible = ref(false);

const breakpoints = useBreakpoints(breakpointsTailwind);
const isMobile = breakpoints.smaller('md');

watch(isMobile, (newIsMobile) => {
  if (newIsMobile) {
    sidebarVisible.value = false;
  }
});
const route = useRoute();
const showSidebars = computed(() => {
  return route.meta.noAuth !== true;
});
</script>

<template>
  <AppFrame>
    <template v-if="showSidebars" #top>
      <SideBar :visible="isMobile" direction="top">
        <Button
          size="small"
          variant="text"
          severity="secondary"
          icon-only
          @click="sidebarVisible = !sidebarVisible"
        >
          <Icon :name="sidebarVisible ? 'close' : 'menu'" />
        </Button>
      </SideBar>
    </template>
    <template v-if="showSidebars" #left>
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
  </AppFrame>
</template>
