<script setup lang="ts">
import type { Tournament } from '@/api/tournaments.ts';
import { timeControlToString } from '@/utils/time.ts';
import { Button, Card } from '@tak-ui-lib/components';
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
  <Card>
    <div class="flex gap-2 items-start">
      <img
        class="w-18 h-18 rounded-md overflow-hidden"
        :src="'/fallback/default_user.webp'"
        alt="Tournament Image"
      />
      <div class="flex flex-col grow">
        <p class="font-semibold text-lg">{{ tournament.metadata.name }}</p>
        <p class="text-sm">Description</p>
      </div>
      <Button size="small" icon-only variant="text" severity="secondary" @click="$emit('click')">
        <LuEye />
      </Button>
    </div>

    <div class="flex flex-wrap gap-x-6 gap-y-2 justify-start items-center">
      <div class="flex items-center gap-2 justify-start">
        <LuSwords />
        {{ tournamentModeToString(tournament) }}
      </div>
      <div class="flex items-center gap-2 justify-start">
        <Fa6ChessBoard />
        {{ tournament.metadata.matchSettings.gameSettings.boardSize }}x{{
          tournament.metadata.matchSettings.gameSettings.boardSize
        }}
      </div>
      <div class="flex items-center gap-2 justify-start">
        <LuClock />
        {{ timeControlToString(tournament.metadata.matchSettings.gameSettings.timeSettings) }}
      </div>
      <div class="flex items-center gap-2 justify-start">
        <LuScale />
        {{ tournament.metadata.matchSettings.gameSettings.halfKomi * 0.5 }} komi
      </div>
    </div>
  </Card>
</template>
