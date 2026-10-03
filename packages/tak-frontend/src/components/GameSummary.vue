<script setup lang="ts">
import type { GameMetadata } from '@/api/game';
import { getDefaultReserve, type TakGameState, type TakPlayer } from '@/tak-core/index.ts';
import { timeControlToString } from '@/utils/time.ts';
import { Button, Card, Tag } from '@tak-ui-lib/components';
import { computed } from 'vue';
import { Fa6ChessBoard, Fa6RegChessPawn, Fa6RegChessQueen } from 'vue-icons-plus/fa6';
import { LuCalendar, LuClock, LuEye, LuPlay, LuScale } from 'vue-icons-plus/lu';
import PlayerLabel from './PlayerLabel.vue';

const props = defineProps<{
  gameMetadata: GameMetadata;
  result: TakGameState;
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

const resultStr = computed(() => {
  switch (props.result.type) {
    case 'ongoing':
      return 'Ongoing';
    case 'draw':
      return 'Draw';
    case 'win':
      return `${resultOfPlayer(props.result, 'white')}-${resultOfPlayer(props.result, 'black')}`;
    case 'aborted':
      return 'Aborted';
  }
});

const isFlatsSpecial = computed(() => {
  const { boardSize, pieces } = props.gameMetadata.gameSettings;
  return getDefaultReserve(boardSize).pieces !== pieces;
});

const isCapstonesSpecial = computed(() => {
  const { boardSize, capstones } = props.gameMetadata.gameSettings;
  return getDefaultReserve(boardSize).capstones !== capstones;
});

const isOpeningSpecial = computed(() => {
  return props.gameMetadata.gameSettings.opening !== 'swap';
});

const openingNames: Record<string, string | undefined> = {
  swap: 'Swap',
  noSwap: 'No Swap',
  doubleStack: 'Double Stack',
};
</script>
<template>
  <Card>
    <div class="flex flex-col gap-2">
      <h3 class="flex items-center gap-2 justify-start">
        <span>Game #{{ gameMetadata.id }}</span>
        <Tag class="flex items-center justify-center font-mono" :label="resultStr" />
      </h3>
      <PlayerLabel :pid="gameMetadata.playerIds.white" type="player"></PlayerLabel>
      <PlayerLabel :pid="gameMetadata.playerIds.black" type="player"></PlayerLabel>
      <template v-if="hideGameSettings !== true">
        <div class="flex items-center gap-2 justify-start">
          <LuCalendar />
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
        <div class="flex items-center gap-2 justify-start">
          <Fa6ChessBoard />
          {{ gameMetadata.gameSettings.boardSize }}x{{ gameMetadata.gameSettings.boardSize }}
        </div>
        <div class="flex items-center gap-2 justify-start">
          <LuClock />
          {{ timeControlToString(gameMetadata.gameSettings.timeSettings) }}
        </div>
        <div class="flex items-center gap-2 justify-start">
          <LuScale />
          {{ gameMetadata.gameSettings.halfKomi * 0.5 }} komi
        </div>
        <div v-if="isFlatsSpecial" class="flex items-center gap-2 justify-start">
          <Fa6RegChessPawn />
          {{ gameMetadata.gameSettings.pieces }} Flat{{
            gameMetadata.gameSettings.pieces !== 1 ? 's' : ''
          }}
        </div>
        <div v-if="isCapstonesSpecial" class="flex items-center gap-2 justify-start">
          <Fa6RegChessQueen />
          {{ gameMetadata.gameSettings.capstones }} Capstone{{
            gameMetadata.gameSettings.capstones !== 1 ? 's' : ''
          }}
        </div>
        <div v-if="isOpeningSpecial" class="flex items-center gap-2 justify-start">
          <LuPlay />
          {{ openingNames[gameMetadata.gameSettings.opening] }}
        </div>
      </template>
    </div>
    <div class="grow flex items-end justify-end">
      <Button
        size="small"
        severity="secondary"
        variant="text"
        :label="result.type === 'ongoing' ? 'Watch' : 'View Result'"
        @click="$emit('click')"
      >
        <template #icon>
          <LuEye />
        </template>
      </Button>
    </div>
  </Card>
</template>
