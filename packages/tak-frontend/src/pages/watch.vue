<script setup lang="ts">
import { useGames } from '@/api/game';
import { useGameHistory, type GameHistory } from '@/api/gameHistory';
import GameSummary from '@/components/GameSummary.vue';
import { gameResultFromString } from '@/tak-core/ptn';
import Paginator from 'primevue/paginator';
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';

const { data: games } = useGames();

const router = useRouter();

function onWatchGame(gameId: string) {
  void router.push(`/online/${gameId}`);
}

const first = ref(0);
const rows = ref(20);

const { data: gameHistory } = useGameHistory(() => ({
  page: Math.floor(first.value / rows.value) + 1,
  pageSize: rows.value,
}));

const lastGameHistory = computed<GameHistory | undefined>((prevGames) => {
  if (!gameHistory.value) return prevGames;
  return gameHistory.value;
});
</script>
<template>
  <div class="w-full mx-auto max-w-4xl p-2 pt-4 flex flex-col gap-6">
    <div class="flex flex-col gap-2">
      <h1 class="text-2xl font-semibold">Live Games</h1>
      <GameSummary
        v-for="game in games"
        :key="game.id"
        :game-metadata="game"
        @click="onWatchGame(game.id)"
      ></GameSummary>
      <p v-if="!games?.length" class="text-muted-color">No live games available.</p>
    </div>
    <div class="flex flex-col gap-2">
      <h1 class="text-2xl font-semibold">Past Games</h1>
      <GameSummary
        v-for="game in lastGameHistory?.items"
        :key="game.id"
        :game-metadata="game"
        :result="gameResultFromString(game.result ?? '') ?? { type: 'ongoing' }"
        @click="onWatchGame(game.id)"
      ></GameSummary>
      <Paginator
        v-model:first="first"
        v-model:rows="rows"
        :total-records="lastGameHistory?.totalCount"
      />
    </div>
  </div>
</template>
