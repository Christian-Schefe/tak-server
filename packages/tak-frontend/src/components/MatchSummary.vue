<script setup lang="ts">
import type { MatchDetail } from '@/api/match.ts';
import { timeControlToString } from '@/utils/time.ts';
import Button from 'primevue/button';
import { Fa6ChessBoard } from 'vue-icons-plus/fa6';
import { LuClock, LuEye, LuScale } from 'vue-icons-plus/lu';
import GameSettingsPopover from './GameSettingsPopover.vue';
import PlayerLabel from './PlayerLabel.vue';

defineProps<{
  matchDetail: MatchDetail;
  hideGameSettings?: boolean;
}>();

defineEmits<{
  click: [];
}>();
</script>
<template>
  <div class="grow flex flex-col gap-2 p-2 bg-content rounded-md">
    <div class="flex gap-2">
      <div class="w-8 h-18 grid grid-rows-2 rounded-md overflow-hidden text-sm">
        <div
          v-for="(res, index) in [matchDetail.player1.score, matchDetail.player2.score]"
          :key="index"
          class="flex items-center justify-center font-mono bg-surface-200 dark:bg-surface-700"
        >
          {{ res }}
        </div>
      </div>
      <div class="flex flex-col gap-2 justify-center grow">
        <PlayerLabel :pid="matchDetail.player1.playerId" type="player"></PlayerLabel>
        <PlayerLabel :pid="matchDetail.player2.playerId" type="player"></PlayerLabel>
      </div>
      <GameSettingsPopover
        v-if="hideGameSettings !== true"
        :settings="matchDetail.settings.gameSettings"
      />
      <Button class="w-8! h-8! p-1!" severity="secondary" @click="$emit('click')">
        <template #icon>
          <LuEye />
        </template>
      </Button>
    </div>

    <div
      v-if="hideGameSettings !== true"
      class="flex flex-wrap gap-x-6 gap-y-2 justify-start items-center"
    >
      <div class="flex items-center gap-2 justify-start">
        <Fa6ChessBoard class="text-primary" />
        {{ matchDetail.settings.gameSettings.boardSize }}x{{
          matchDetail.settings.gameSettings.boardSize
        }}
      </div>
      <div class="flex items-center gap-2 justify-start">
        <LuClock class="text-primary" />
        {{ timeControlToString(matchDetail.settings.gameSettings.timeSettings) }}
      </div>
      <div class="flex items-center gap-2 justify-start">
        <LuScale class="text-primary" />
        {{ matchDetail.settings.gameSettings.halfKomi * 0.5 }} komi
      </div>
    </div>
  </div>
</template>
