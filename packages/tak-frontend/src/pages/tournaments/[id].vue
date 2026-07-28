<script setup lang="ts">
import { useAccount } from '@/api/auth';
import { useMatches } from '@/api/match';
import {
  useDeregisterFromTournament,
  useFinishTournament,
  useRegisterForTournament,
  useSetRegistrationOpen,
  useStartNextRound,
  useStartTournament,
  useTournament,
} from '@/api/tournaments';
import MatchSummary from '@/components/MatchSummary.vue';
import PlayerLabel from '@/components/PlayerLabel.vue';
import TournamentSummary from '@/components/TournamentSummary.vue';
import Button from 'primevue/button';
import Tab from 'primevue/tab';
import TabList from 'primevue/tablist';
import Tabs from 'primevue/tabs';
import Tag from 'primevue/tag';
import { computed, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

const route = useRoute('/tournaments/[id]');
const { data: tournament } = useTournament(() => route.params.id);

const tournamentPlayers = computed(() => {
  const players = tournament.value?.players.map((player) => {
    return {
      id: player.id,
      rank: 0,
      score: player.score,
    };
  });
  players?.sort((a, b) => (a.score !== b.score ? b.score - a.score : a.id.localeCompare(b.id)));
  players?.forEach((player, index) => {
    const prevPlayer = players[index - 1];
    player.rank = prevPlayer && prevPlayer.score === player.score ? prevPlayer.rank : index + 1;
  });
  return players;
});

const currentRound = ref<string | number>(0);

const roundMatchIds = computed(() => {
  if (typeof currentRound.value !== 'number') {
    return [];
  }
  return tournament.value?.rounds[currentRound.value]?.matches ?? [];
});
const matches = useMatches(roundMatchIds);

const roundData = computed(() => {
  if (typeof currentRound.value !== 'number') {
    return null;
  }
  const data = tournament.value?.rounds[currentRound.value];
  if (!data) {
    return null;
  }
  return data.matches
    .map((matchId) => {
      const matchData = matches.value[matchId];
      if (!matchData) return null;
      return matchData;
    })
    .filter((match): match is NonNullable<typeof match> => match !== null);
  /* return roundData.concat(
    data.byes.map((playerId) => ({
      id: '',
      status: 'completed',
      player1Id: playerId,
      score: 'Bye',
    })),
  );*/
});

const { mutate: registerForTournament } = useRegisterForTournament();
const { mutate: leaveTournament } = useDeregisterFromTournament();
const { mutate: startTournament } = useStartTournament();
const { mutate: finishTournament } = useFinishTournament();
const { mutate: startNextRound } = useStartNextRound();
const { mutate: setRegistrationOpen } = useSetRegistrationOpen();

const { data: account } = useAccount();
const isSignedUp = computed(() => {
  return tournament.value?.players.some((player) => player.id === account.value?.playerId);
});

function onRegister() {
  registerForTournament(route.params.id);
}

function onLeave() {
  leaveTournament(route.params.id);
}

function onStart() {
  startTournament(route.params.id);
}

function onFinish() {
  finishTournament(route.params.id);
}

function onStartNextRound() {
  startNextRound(route.params.id);
}

const router = useRouter();
function goToMatch(matchId: string) {
  void router.push(`/match/${matchId}`);
}

const tournamentStatusLabels = {
  upcoming: 'Upcoming',
  ongoing: 'Ongoing',
  completed: 'Completed',
};
const tournamentStatusSeverities = {
  upcoming: 'info',
  ongoing: 'warn',
  completed: 'success',
};
</script>
<template>
  <div class="w-full mx-auto max-w-4xl p-2 pt-4 flex flex-col gap-6">
    <div v-if="tournament" class="flex flex-col gap-2">
      <div class="flex items-center gap-4">
        <h1 class="text-2xl font-semibold">{{ tournament.metadata.name }}</h1>
        <Tag
          :value="tournamentStatusLabels[tournament.status.type]"
          :severity="tournamentStatusSeverities[tournament.status.type]"
        />
        <div class="grow"></div>
      </div>

      <TournamentSummary :tournament="tournament" />

      <div v-if="tournament.status.type === 'upcoming'" class="flex gap-2">
        <Button
          v-if="tournament.status.registrationOpen && !isSignedUp"
          label="Enter Tournament"
          @click="onRegister"
        />
        <Button
          v-else-if="tournament.status.registrationOpen && isSignedUp"
          label="Leave Tournament"
          severity="danger"
          @click="onLeave"
        />
        <Button v-else severity="secondary" label="Registration is closed" disabled />
      </div>
    </div>

    <div v-if="tournament" class="flex flex-col gap-2">
      <h1 class="text-2xl font-semibold">Players</h1>
      <div
        class="items-center grid gap-x-4 gap-y-2"
        :style="{ gridTemplateColumns: 'auto 1fr auto' }"
      >
        <p class="font-semibold">Rank</p>
        <p class="font-semibold">Player</p>
        <p class="font-semibold">Score</p>
        <template v-for="(player, index) in tournamentPlayers" :key="player.id">
          <span class="text-lg font-semibold text-primary font-mono">#{{ index + 1 }}</span>
          <PlayerLabel :pid="player.id" type="player" :show-rating="false"></PlayerLabel>
          <span class="ml-auto font-mono">{{ player.score }}</span>
        </template>
      </div>
    </div>

    <div v-if="tournament && tournament.status.type !== 'upcoming'" class="flex flex-col gap-2">
      <h1 class="text-2xl font-semibold">Rounds</h1>
      <div class="rounded-md overflow-hidden">
        <Tabs v-model:value="currentRound">
          <TabList>
            <Tab v-for="(_, index) in tournament.rounds" :key="index" :value="index">
              Round {{ index + 1 }}
            </Tab>
          </TabList>
        </Tabs>
      </div>
      <MatchSummary
        v-for="match in roundData"
        :key="match.id"
        :match-detail="match"
        hide-game-settings
        @click="goToMatch(match.id)"
      />
    </div>

    <div
      v-if="tournament && tournament.status.type !== 'completed' && account?.isAdmin === true"
      class="flex flex-col gap-2"
    >
      <h1 class="text-2xl font-semibold">Actions</h1>
      <Button
        v-if="tournament.status.type === 'upcoming'"
        label="Start Tournament"
        @click="onStart"
      />
      <Button
        v-if="tournament.status.type === 'ongoing'"
        label="Start Next Round"
        @click="onStartNextRound"
      />
      <Button
        v-if="tournament.status.type === 'ongoing'"
        label="Finish Tournament"
        @click="onFinish"
      />
      <Button
        v-if="tournament.status.type === 'upcoming'"
        :label="tournament.status.registrationOpen ? 'Close Registration' : 'Open Registration'"
        @click="
          setRegistrationOpen({
            tournamentId: route.params.id,
            open: !tournament.status.registrationOpen,
          })
        "
      />
    </div>
  </div>
</template>
