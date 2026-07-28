<script setup lang="ts">
import { useIsAccountOnline } from '@/api/account';
import { useAccount } from '@/api/auth';
import { useMatch, useMatchGames, useMatchReadiness, useMatchSetPlayerReady } from '@/api/match';
import { usePlayerInfo } from '@/api/player';
import GameSummary from '@/components/GameSummary.vue';
import MatchReadyTagOrButton from '@/components/MatchReadyTagOrButton.vue';
import MatchSummary from '@/components/MatchSummary.vue';
import { gameResultFromString } from '@/tak-core/ptn';
import Tag from 'primevue/tag';
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';

const route = useRoute('/match.[id]');

const { data: account } = useAccount();

const isOwnMatch = computed(
  () =>
    account.value !== undefined &&
    (match.value?.player1.playerId === account.value.playerId ||
      match.value?.player2.playerId === account.value.playerId),
);

const { data: match } = useMatch(() => route.params.id);
const { data: matchReadiness } = useMatchReadiness(() => route.params.id, isOwnMatch);
const { data: matchGames } = useMatchGames(() => route.params.id);

const setPlayerReady = useMatchSetPlayerReady();

const isSelfReady = computed(
  () => account.value !== undefined && matchReadiness.value?.readyPlayer === account.value.playerId,
);

const { data: player1Info } = usePlayerInfo(() => match.value?.player1.playerId);
const { data: player2Info } = usePlayerInfo(() => match.value?.player2.playerId);

const isPlayer1Online = useIsAccountOnline(() => player1Info.value?.accountId);
const isPlayer2Online = useIsAccountOnline(() => player2Info.value?.accountId);

function toggleReady() {
  if (!matchReadiness.value) return;
  setPlayerReady.mutate({ id: route.params.id, ready: !isSelfReady.value });
}

const router = useRouter();
function goToGame(gameId: string) {
  void router.push(`/online/${gameId}`);
}
</script>
<template>
  <div class="w-full mx-auto max-w-4xl p-2 pt-4 flex flex-col gap-6">
    <div v-if="match" class="flex flex-col gap-2">
      <div class="flex items-center gap-4">
        <h1 class="text-2xl font-semibold">Match</h1>
        <Tag
          :value="match.status === 'completed' ? 'Completed' : 'Ongoing'"
          :severity="match.status === 'completed' ? 'success' : 'warn'"
        />
      </div>
      <MatchSummary :match-detail="match"></MatchSummary>

      <div v-if="match.status === 'waiting' && isOwnMatch" class="grid grid-cols-2 gap-2">
        <MatchReadyTagOrButton
          :is-online="isPlayer1Online ?? false"
          :is-ready="matchReadiness?.readyPlayer === match.player1.playerId"
          :is-button="account !== undefined && match.player1.playerId === account.playerId"
          @toggle-ready="toggleReady"
        />
        <MatchReadyTagOrButton
          :is-online="isPlayer2Online ?? false"
          :is-ready="matchReadiness?.readyPlayer === match.player2.playerId"
          :is-button="account !== undefined && match.player2.playerId === account.playerId"
          @toggle-ready="toggleReady"
        />
      </div>
    </div>
    <div v-if="match" class="flex flex-col gap-2">
      <h1 class="text-2xl font-semibold">Games</h1>
      <GameSummary
        v-for="game in matchGames"
        :key="game.id"
        :game-metadata="game"
        :result="gameResultFromString(game.result ?? '') ?? { type: 'ongoing' }"
        hide-game-settings
        @click="goToGame(game.id)"
      ></GameSummary>
      <p v-if="!matchGames?.length" class="text-muted-color">No live games available.</p>
    </div>
  </div>
</template>
