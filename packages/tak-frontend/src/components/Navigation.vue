<script setup lang="ts">
import { useAccount } from '@/api/auth';
import PlayerLabel from '@/components/PlayerLabel.vue';
import { breakpointsTailwind, useBreakpoints } from '@vueuse/core';
import Button from 'primevue/button';
import Drawer from 'primevue/drawer';
import { computed, ref, watch } from 'vue';
import {
  LuLogIn,
  LuMenu,
  LuPlay,
  LuPuzzle,
  LuSettings,
  LuTrophy,
  LuTv,
  LuUser,
  LuZoomIn,
  LuUsers,
  LuMedal,
} from 'vue-icons-plus/lu';
import Divider from 'primevue/divider';
import Badge from 'primevue/badge';
import { useGames } from '@/api/game';
import { useSeeks } from '@/api/seek';

type MenuItem =
  | {
      type: undefined;
      label: string;
      icon: string;
      routerLink: string;
      badge?: string;
    }
  | {
      type: 'separator';
    };

const { data: account } = useAccount();

const icons: Record<string, unknown> = {
  play: LuPlay,
  watch: LuTv,
  community: LuUsers,
  puzzle: LuPuzzle,
  tournament: LuTrophy,
  settings: LuSettings,
  account: LuUser,
  analysis: LuZoomIn,
  login: LuLogIn,
  leaderboard: LuMedal,
};

const { data: seeks } = useSeeks();
const { data: games } = useGames();

const opponentSeekCount = computed(() => {
  if (!seeks.value || !account.value) return undefined;
  return seeks.value.filter((seek) => seek.creatorId !== account.value.playerId).length;
});

const items = computed<MenuItem[]>(() => {
  return [
    {
      label: 'Play',
      icon: 'play',
      routerLink: '/play',
      badge:
        opponentSeekCount.value !== undefined
          ? `${opponentSeekCount.value.toString()} Seeks`
          : undefined,
    },
    {
      label: 'Watch',
      icon: 'watch',
      routerLink: '/watch',
      badge: games.value ? `${games.value.length.toString()} Games` : undefined,
    },
    {
      label: 'Analysis',
      icon: 'analysis',
      routerLink: '/analysis',
    },
    {
      label: 'Community',
      icon: 'community',
      routerLink: '/community',
    },
    {
      label: 'Leaderboard',
      icon: 'leaderboard',
      routerLink: '/leaderboard',
    },
    {
      label: 'Puzzles',
      icon: 'puzzle',
      routerLink: '/puzzle',
    },
    {
      label: 'Tournaments',
      icon: 'tournament',
      routerLink: '/tournaments',
    },
    {
      type: 'separator',
    },
    {
      label: 'Settings',
      icon: 'settings',
      routerLink: '/settings',
    },
    ...(account.value !== undefined && !account.value.isGuest
      ? [
          {
            label: 'Account',
            icon: 'account',
            routerLink: '/account',
          },
        ]
      : []),
    ...(account.value !== undefined && account.value.isGuest
      ? [
          {
            label: 'Login',
            icon: 'login',
            routerLink: '/login',
          },
        ]
      : []),
  ];
});

const breakpoint = useBreakpoints(breakpointsTailwind);

const drawerVisibleToggle = ref(false);
const drawerToggleable = breakpoint.smaller('lg');
const drawerVisible = computed(() => {
  if (drawerToggleable.value) {
    return drawerVisibleToggle.value;
  }
  return true;
});
watch(drawerToggleable, (isSmaller) => {
  if (!isSmaller) {
    drawerVisibleToggle.value = false;
  }
});
</script>

<template>
  <div
    class="w-full h-full flex items-center lg:hidden border-b bg-content border-surface z-10000 relative"
  >
    <Button
      class="aspect-square! h-full!"
      severity="secondary"
      variant="text"
      @click="drawerVisibleToggle = !drawerVisibleToggle"
    >
      <LuMenu class="w-5 h-5" />
    </Button>
    <RouterLink class="text-2xl font-bold ml-2 my-2" to="/" :draggable="false">Playtak</RouterLink>
  </div>
  <Drawer
    :visible="drawerVisible"
    :modal="drawerToggleable"
    class="w-54!"
    @update:visible="drawerVisibleToggle = $event"
  >
    <template #container>
      <RouterLink v-if="!drawerToggleable" class="p-2 w-full" to="/">
        <div class="w-full p-2">
          <img class="w-full pt-2 px-4 dark:invert" src="/logo.svg" />
        </div>
      </RouterLink>
      <div v-else class="h-12"></div>
      <div class="grow flex flex-col p-2 gap-1 items-stretch">
        <template v-for="(item, index) in items" :key="index">
          <Button v-slot="slotProps" as-child variant="text" severity="contrast">
            <RouterLink
              v-if="item.type !== 'separator'"
              v-ripple
              :class="slotProps.class"
              class="flex items-center gap-2 cursor-pointer"
              :to="item.routerLink"
              :draggable="false"
            >
              <component :is="icons[item.icon]" class="mr-2 w-5 h-5"></component>
              <p class="grow text-left">
                {{ item.label }}
              </p>
              <Badge v-if="item.badge" :value="item.badge" size="small"></Badge>
            </RouterLink>
            <div v-else class="px-2">
              <Divider />
            </div>
          </Button>
        </template>
        <div class="grow"></div>
        <div class="p-2">
          <PlayerLabel
            v-if="account"
            :pid="account.accountId"
            type="account"
            :show-rating="false"
          />
        </div>
      </div>
    </template>
  </Drawer>
</template>
