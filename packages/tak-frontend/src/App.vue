<script setup lang="ts">
import { useNavigateOnGamesStartWebSocketSubscription } from './api/game.ts';
import Navigation from './components/Navigation.vue';
import { useAuthStore } from './features/auth.ts';
import { useWebSocketStore } from './features/websocket.ts';

useAuthStore();
useWebSocketStore();
useNavigateOnGamesStartWebSocketSubscription();
</script>

<template>
  <div class="absolute left-0 top-0 max-lg:right-0 max-lg:h-12 lg:bottom-0 lg:w-54 text-color">
    <Navigation></Navigation>
  </div>
  <div
    class="absolute flex flex-col items-center left-0 top-0 right-0 bottom-0 max-lg:top-12 lg:left-54 text-color overflow-hidden overflow-y-auto"
  >
    <RouterView></RouterView>
  </div>
</template>
