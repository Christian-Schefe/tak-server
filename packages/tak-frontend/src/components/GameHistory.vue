<script setup lang="ts">
import { usePlayerGameHistory, type GameHistory } from '@/api/gameHistory';
import GameSummary from '@/components/GameSummary.vue';
import { gameResultFromString } from '@/tak-core/ptn';
import { Paginator } from '@tak-ui-lib/components';
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';

const props = defineProps<{
  playerId: string;
}>();

const router = useRouter();

function onWatchGame(gameId: string) {
  void router.push(`/online/${gameId}`);
}

const currentPage = ref(1);
const pageSize = 6;

const { data: gameHistory } = usePlayerGameHistory(
  () => props.playerId,
  () => ({
    page: currentPage.value,
    pageSize,
  }),
);

const lastGameHistory = computed<GameHistory | undefined>((prevGames) => {
  if (!gameHistory.value) return prevGames;
  return gameHistory.value;
});
</script>
<template>
  <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
    <template v-for="game in lastGameHistory?.items" :key="game.id">
      <GameSummary
        class="min-w-70"
        :game-metadata="game"
        :result="gameResultFromString(game.result ?? '') ?? { type: 'ongoing' }"
        @click="onWatchGame(game.id)"
      ></GameSummary>
    </template>
  </div>
  <div class="flex items-center justify-center">
    <Paginator
      v-model="currentPage"
      :items-per-page="pageSize"
      :total-items="lastGameHistory?.totalCount ?? 0"
    />
  </div>
</template>
