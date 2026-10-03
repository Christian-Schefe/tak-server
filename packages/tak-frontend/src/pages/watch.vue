<script setup lang="ts">
import { useGames } from '@/api/game';
import GameSummary from '@/components/GameSummary.vue';
import Page from '@/components/Page.vue';
import { useRouter } from 'vue-router';

const { data: games } = useGames();

const router = useRouter();

function onWatchGame(gameId: string) {
  void router.push(`/online/${gameId}`);
}
</script>
<template>
  <Page>
    <h1 class="text-2xl font-semibold">Live Games</h1>
    <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
      <GameSummary
        v-for="game in games"
        :key="game.id"
        :game-metadata="game"
        :result="{ type: 'ongoing' }"
        @click="onWatchGame(game.id)"
      ></GameSummary>
    </div>
    <p v-if="!games?.length">No live games available.</p>
  </Page>
</template>
