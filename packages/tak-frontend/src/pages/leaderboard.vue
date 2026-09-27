<script setup lang="ts">
import { usePlayerLeaderboard } from '@/api/player';
import Page from '@/components/Page.vue';
import PlayerLabel from '@/components/PlayerLabel.vue';
import { Paginator } from '@tak-ui-lib/components';
import { ref } from 'vue';

const currentPage = ref(1);
const pageSize = 20;

const { data: playerData } = usePlayerLeaderboard(() => ({
  page: currentPage.value,
  pageSize,
}));
</script>
<template>
  <Page>
    <h1 class="text-2xl font-semibold">Leaderboard</h1>
    <div
      class="items-center grid gap-x-4 gap-y-2"
      :style="{ gridTemplateColumns: 'auto 1fr auto' }"
    >
      <p class="font-semibold">Rank</p>
      <p class="font-semibold">Player</p>
      <p class="font-semibold">Rating</p>
      <template v-for="(player, index) in playerData?.items" :key="player.playerId">
        <span class="text-lg font-semibold text-primary font-mono"
          >#{{ index + 1 + (currentPage - 1) * pageSize }}</span
        >
        <PlayerLabel :pid="player.playerId" type="player" :show-rating="false"></PlayerLabel>
        <span class="ml-auto font-mono">{{ player.rating.toFixed(0) }}</span>
      </template>
    </div>
    <div class="flex items-center justify-center">
      <Paginator
        v-model="currentPage"
        :total-items="playerData?.totalCount ?? 0"
        :items-per-page="pageSize"
      />
    </div>
  </Page>
</template>
