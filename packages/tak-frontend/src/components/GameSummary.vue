<script setup lang="ts">
import type { GameMetadata } from '@/api/game';
import { type TakGameState, type TakPlayer } from '@/tak-core/index.ts';
import { timeControlToString } from '@/utils/time.ts';
import Button from 'primevue/button';
import { computed } from 'vue';
import { Fa6ChessBoard } from 'vue-icons-plus/fa6';
import { LuCalendar, LuClock, LuEye, LuScale } from 'vue-icons-plus/lu';
import GameSettingsPopover from './GameSettingsPopover.vue';
import PlayerLabel from './PlayerLabel.vue';

const props = defineProps<{
  gameMetadata: GameMetadata;
  result?: TakGameState;
  hideGameSettings?: boolean;
}>();

defineEmits<{
  click: [];
}>();

function resultOfPlayer(result: TakGameState, player: TakPlayer) {
  switch (result.type) {
    case 'ongoing':
      return '';
    case 'draw':
      return '1/2';
    case 'win':
      if (result.winner === player) {
        switch (result.reason) {
          case 'flats':
            return 'F';
          case 'road':
            return 'R';
          default:
            return '1';
        }
      } else {
        return '0';
      }
    case 'aborted':
      return '0';
  }
}

const resultArr = computed(() => {
  if (!props.result) return undefined;
  return [resultOfPlayer(props.result, 'white'), resultOfPlayer(props.result, 'black')];
});

const resultColor: Record<string, string | undefined> = {
  '1': 'bg-primary text-primary-contrast',
  F: 'bg-primary text-primary-contrast',
  R: 'bg-primary text-primary-contrast',
  '0': 'bg-surface-200 dark:bg-surface-700',
  '1/2': 'bg-surface-200 dark:bg-surface-700',
  '': '',
};
</script>
<template>
  <div class="grow flex flex-col gap-2 p-2 bg-content rounded-md">
    <div class="flex gap-2">
      <div v-if="resultArr" class="w-8 h-18 grid grid-rows-2 rounded-md overflow-hidden text-sm">
        <div
          v-for="(res, index) in resultArr"
          :key="index"
          :class="resultColor[res]"
          class="flex items-center justify-center font-mono"
        >
          {{ res }}
        </div>
      </div>
      <div class="flex flex-col gap-2 justify-center grow">
        <PlayerLabel :pid="gameMetadata.playerIds.white" type="player"></PlayerLabel>
        <PlayerLabel :pid="gameMetadata.playerIds.black" type="player"></PlayerLabel>
      </div>
      <GameSettingsPopover v-if="hideGameSettings !== true" :settings="gameMetadata.gameSettings" />
      <Button class="w-8! h-8! p-1!" severity="secondary" @click="$emit('click')">
        <template #icon>
          <LuEye />
        </template>
      </Button>
    </div>

    <div
      v-if="hideGameSettings !== true || resultArr"
      class="flex flex-wrap gap-x-6 gap-y-2 justify-start items-center"
    >
      <div v-if="resultArr" class="flex items-center gap-2 justify-start">
        <LuCalendar class="text-primary" />
        {{
          new Date(gameMetadata.date).toLocaleDateString([], {
            month: 'short',
            day: 'numeric',
            year: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
          })
        }}
      </div>
      <div v-if="hideGameSettings !== true" class="flex items-center gap-2 justify-start">
        <Fa6ChessBoard class="text-primary" />
        {{ gameMetadata.gameSettings.boardSize }}x{{ gameMetadata.gameSettings.boardSize }}
      </div>
      <div v-if="hideGameSettings !== true" class="flex items-center gap-2 justify-start">
        <LuClock class="text-primary" />
        {{ timeControlToString(gameMetadata.gameSettings.timeSettings) }}
      </div>
      <div v-if="hideGameSettings !== true" class="flex items-center gap-2 justify-start">
        <LuScale class="text-primary" />
        {{ gameMetadata.gameSettings.halfKomi * 0.5 }} komi
      </div>
    </div>
  </div>
</template>
