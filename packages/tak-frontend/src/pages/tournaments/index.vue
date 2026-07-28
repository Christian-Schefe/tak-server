<script setup lang="ts">
import { useCreateTournament, useTournaments } from '@/api/tournaments';
import TournamentSummary from '@/components/TournamentSummary.vue';
import Button from 'primevue/button';
import { useRouter } from 'vue-router';

const { data: tournaments } = useTournaments();

const { mutate: createTournamentMutation } = useCreateTournament();

function createTournament() {
  createTournamentMutation({
    name: 'Test Tournament',
    matchSettings: {
      gameSettings: {
        boardSize: 6,
        pieces: 30,
        capstones: 1,
        halfKomi: 0,
        timeSettings: {
          type: 'realtime',
          contingentMs: 1000 * 60 * 10,
          incrementMs: 1000 * 5,
          extra: null,
        },
        opening: 'swap',
      },
      isRated: true,
      matchMode: {
        type: 'fixedGames',
        games: 2,
      },
    },
    tournamentFormat: {
      type: 'roundRobin',
    },
  });
}

const router = useRouter();

function goToTournament(tournamentId: string) {
  void router.push(`/tournaments/${tournamentId}`);
}
</script>
<template>
  <div class="w-full mx-auto max-w-4xl p-2 pt-4 flex flex-col gap-6">
    <div class="flex flex-col gap-2">
      <h1 class="text-2xl font-semibold">Tournaments</h1>
      <TournamentSummary
        v-for="tournament in tournaments"
        :key="tournament.metadata.id"
        :tournament="tournament"
        @click="goToTournament(tournament.metadata.id)"
      ></TournamentSummary>
      <p v-if="!tournaments?.length" class="text-muted-color">No tournaments available.</p>
      <Button severity="secondary" @click="createTournament()">Create Test Tournament</Button>
    </div>
  </div>
</template>
