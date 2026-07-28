<script setup lang="ts">
import { usePlayerLeaderboard } from '@/api/player';
import PlayerLabel from '@/components/PlayerLabel.vue';
import Paginator from 'primevue/paginator';
import { ref } from 'vue';

const first = ref(0);
const rows = ref(20);

const { data: playerData } = usePlayerLeaderboard(() => ({
  page: Math.floor(first.value / rows.value) + 1,
  pageSize: rows.value,
}));
</script>
<template>
  <div class="w-full mx-auto max-w-4xl p-2 pt-4 flex flex-col gap-6">
    <div class="flex flex-col gap-2">
      <h1 class="text-2xl font-semibold">Leaderboard</h1>
      <div
        class="items-center grid gap-x-4 gap-y-2"
        :style="{ gridTemplateColumns: 'auto 1fr auto' }"
      >
        <p class="font-semibold">Rank</p>
        <p class="font-semibold">Player</p>
        <p class="font-semibold">Rating</p>
        <template v-for="(player, index) in playerData?.items" :key="player.playerId">
          <span class="text-lg font-semibold text-primary font-mono">#{{ index + 1 + first }}</span>
          <PlayerLabel :pid="player.playerId" type="player" :show-rating="false"></PlayerLabel>
          <span class="ml-auto font-mono">{{ player.rating.toFixed(0) }}</span>
        </template>
      </div>
      <Paginator
        v-model:first="first"
        v-model:rows="rows"
        :total-records="playerData?.totalCount"
      />
    </div>
  </div>
</template>
