<script setup lang="ts">
import type { Tournament } from '@/api/tournaments.ts';
import { timeControlToString } from '@/utils/time.ts';
import Button from 'primevue/button';
import { Fa6ChessBoard } from 'vue-icons-plus/fa6';
import { LuClock, LuEye, LuScale, LuSwords } from 'vue-icons-plus/lu';

defineProps<{
  tournament: Tournament;
}>();

defineEmits<{
  click: [];
}>();

function tournamentModeToString(tournament: Tournament): string {
  switch (tournament.metadata.tournamentFormat.type) {
    case 'roundRobin':
      return 'Round Robin';
    case 'swiss':
      return `Swiss (${tournament.metadata.tournamentFormat.rounds.toString()} rounds)`;
    default:
      return 'Unknown';
  }
}
</script>
<template>
  <div class="grow flex flex-col gap-2 p-2 bg-content rounded-md">
    <div class="flex gap-2">
      <img
        class="w-18 h-18 rounded-md overflow-hidden"
        :src="'/fallback/default_user.webp'"
        alt="Tournament Image"
      />
      <div class="flex flex-col grow">
        <p class="font-semibold text-lg">{{ tournament.metadata.name }}</p>
        <p class="text-muted-color text-sm">Description</p>
      </div>
      <Button class="w-8! h-8! p-1!" severity="secondary" @click="$emit('click')">
        <template #icon>
          <LuEye />
        </template>
      </Button>
    </div>

    <div class="flex flex-wrap gap-x-6 gap-y-2 justify-start items-center">
      <div class="flex items-center gap-2 justify-start">
        <LuSwords class="text-primary" />
        {{ tournamentModeToString(tournament) }}
      </div>
      <div class="flex items-center gap-2 justify-start">
        <Fa6ChessBoard class="text-primary" />
        {{ tournament.metadata.matchSettings.gameSettings.boardSize }}x{{
          tournament.metadata.matchSettings.gameSettings.boardSize
        }}
      </div>
      <div class="flex items-center gap-2 justify-start">
        <LuClock class="text-primary" />
        {{ timeControlToString(tournament.metadata.matchSettings.gameSettings.timeSettings) }}
      </div>
      <div class="flex items-center gap-2 justify-start">
        <LuScale class="text-primary" />
        {{ tournament.metadata.matchSettings.gameSettings.halfKomi * 0.5 }} komi
      </div>
    </div>
  </div>
</template>
