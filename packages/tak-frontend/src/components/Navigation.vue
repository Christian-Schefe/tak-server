<script setup lang="ts">
import { useAccount } from '@/api/auth';
import { useGames } from '@/api/game';
import { useSeeks } from '@/api/seek';
import { Button } from '@tak-ui-lib/components';
import { computed } from 'vue';
import {
  LuLogIn,
  LuMedal,
  LuPlay,
  LuPuzzle,
  LuSettings,
  LuTrophy,
  LuTv,
  LuUser,
  LuUsers,
  LuZoomIn,
} from 'vue-icons-plus/lu';
import { RouterLink, useRoute } from 'vue-router';
import PlayerLabel from './PlayerLabel.vue';

interface MenuItem {
  label: string;
  icon: string;
  path: string;
  badge?: string;
}

defineEmits<{
  navigate: [];
}>();

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
      path: '/play',
      badge:
        opponentSeekCount.value !== undefined
          ? `${opponentSeekCount.value.toString()} Seeks`
          : undefined,
    },
    {
      label: 'Watch',
      icon: 'watch',
      path: '/watch',
      badge: games.value ? `${games.value.length.toString()} Games` : undefined,
    },
    {
      label: 'Analysis',
      icon: 'analysis',
      path: '/analysis',
    },
    {
      label: 'Community',
      icon: 'community',
      path: '/community',
    },
    {
      label: 'Leaderboard',
      icon: 'leaderboard',
      path: '/leaderboard',
    },
    {
      label: 'Puzzles',
      icon: 'puzzle',
      path: '/puzzle',
    },
    {
      label: 'Tournaments',
      icon: 'tournament',
      path: '/tournaments',
    },
    {
      label: 'Settings',
      icon: 'settings',
      path: '/settings',
    },
    ...(account.value !== undefined && !account.value.isGuest
      ? [
          {
            label: 'Account',
            icon: 'account',
            path: '/account',
          },
        ]
      : []),
    ...(account.value !== undefined && account.value.isGuest
      ? [
          {
            label: 'Login',
            icon: 'login',
            path: '/login',
          },
        ]
      : []),
  ];
});

const route = useRoute();

function isActive(path: string) {
  return route.path === path;
}
</script>

<template>
  <div class="grow flex flex-col gap-2">
    <RouterLink class="p-2 w-full" to="/">
      <div class="w-full p-2">
        <img class="w-full pt-2 px-4 dark:invert" src="/logo.svg" />
      </div>
    </RouterLink>
    <template v-for="(item, index) in items" :key="index">
      <Button
        variant="text"
        :severity="isActive(item.path) ? 'primary' : 'secondary'"
        :as="{ component: RouterLink, props: { to: item.path } }"
        :label="item.label"
        @click="$emit('navigate')"
      >
        <template v-if="item.icon" #icon>
          <component :is="icons[item.icon]" />
        </template>
        <template v-if="item.badge" #icon-append>
          <span class="ml-2">
            <span class="text-xs">{{ item.badge }}</span>
          </span>
        </template>
      </Button>
    </template>
    <div class="grow"></div>
    <div class="p-2">
      <PlayerLabel v-if="account" :pid="account.accountId" type="account" :show-rating="false" />
    </div>
  </div>
</template>
