<script setup lang="ts">
import { useGames } from '@/api/game';
import { useGameHistory, type GameHistory } from '@/api/gameHistory';
import GameSummary from '@/components/GameSummary.vue';
import Page from '@/components/Page.vue';
import { gameResultFromString } from '@/tak-core/ptn';
import { Paginator } from '@tak-ui-lib/components';
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';

const { data: games } = useGames();

const router = useRouter();

function onWatchGame(gameId: string) {
  void router.push(`/online/${gameId}`);
}

const currentPage = ref(1);
const pageSize = 1;

const { data: gameHistory } = useGameHistory(() => ({
  page: currentPage.value,
  pageSize,
}));

const lastGameHistory = computed<GameHistory | undefined>((prevGames) => {
  if (!gameHistory.value) return prevGames;
  return gameHistory.value;
});
</script>
<template>
  <Page>
    <h1 class="text-2xl font-semibold">Live Games</h1>
    <GameSummary
      v-for="game in games"
      :key="game.id"
      :game-metadata="game"
      @click="onWatchGame(game.id)"
    ></GameSummary>
    <p v-if="!games?.length">No live games available.</p>

    <h1 class="text-2xl font-semibold">Past Games</h1>
    <GameSummary
      v-for="game in lastGameHistory?.items"
      :key="game.id"
      :game-metadata="game"
      :result="gameResultFromString(game.result ?? '') ?? { type: 'ongoing' }"
      @click="onWatchGame(game.id)"
    ></GameSummary>
    <div class="flex items-center justify-center">
      <Paginator
        v-model="currentPage"
        :items-per-page="pageSize"
        :total-items="lastGameHistory?.totalCount ?? 0"
      />
    </div>
  </Page>
</template>
