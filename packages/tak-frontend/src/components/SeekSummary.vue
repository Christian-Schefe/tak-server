<script setup lang="ts">
import type { SeekInfo } from '@/api/seek.ts';
import { getDefaultReserve } from '@/tak-core/index.ts';
import { timeControlToString } from '@/utils/time.ts';
import { Button, Card } from '@tak-ui-lib/components';
import { computed } from 'vue';
import { Fa6ChessBoard, Fa6RegChessPawn, Fa6RegChessQueen } from 'vue-icons-plus/fa6';
import { LuClock, LuContrast, LuPlay, LuScale, LuSwords, LuTrash } from 'vue-icons-plus/lu';
import PlayerLabel from './PlayerLabel.vue';

const props = defineProps<{
  seek: SeekInfo;
  action: 'accept' | 'delete';
}>();

defineEmits<{
  click: [];
}>();

const colorNames: Record<string, string | undefined> = {
  black: 'Black',
  white: 'White',
  random: 'Random',
};

const isFlatsSpecial = computed(() => {
  const { boardSize, pieces } = props.seek.gameSettings;
  return getDefaultReserve(boardSize).pieces !== pieces;
});

const isCapstonesSpecial = computed(() => {
  const { boardSize, capstones } = props.seek.gameSettings;
  return getDefaultReserve(boardSize).capstones !== capstones;
});

const isOpeningSpecial = computed(() => {
  return props.seek.gameSettings.opening !== 'swap';
});

const openingNames: Record<string, string | undefined> = {
  swap: 'Swap',
  noSwap: 'No Swap',
  doubleStack: 'Double Stack',
};
</script>
<template>
  <Card>
    <div class="flex gap-2">
      <div class="flex flex-col gap-2 justify-center">
        <PlayerLabel :pid="seek.creatorId" type="player"></PlayerLabel>
      </div>
      <Tag v-if="!seek.isRated" severity="warn">Unrated</Tag>
      <div class="grow" />

      <Button severity="secondary" icon-only @click="$emit('click')">
        <LuTrash v-if="action === 'delete'" />
        <LuSwords v-else />
      </Button>
    </div>

    <div class="flex flex-wrap gap-x-6 gap-y-2 justify-start items-center">
      <div class="flex items-center gap-2 justify-start">
        <LuContrast class="text-primary" />
        {{ colorNames[seek.color] }}
      </div>
      <div class="flex items-center gap-2 justify-start">
        <Fa6ChessBoard class="text-primary" />
        {{ seek.gameSettings.boardSize }}x{{ seek.gameSettings.boardSize }}
      </div>
      <div class="flex items-center gap-2 justify-start">
        <LuScale class="text-primary" />
        {{ seek.gameSettings.halfKomi * 0.5 }} komi
      </div>
      <div class="flex items-center gap-2 justify-start">
        <LuClock class="text-primary" />
        {{ timeControlToString(seek.gameSettings.timeSettings) }}
      </div>
      <div v-if="isFlatsSpecial" class="flex items-center gap-2 justify-start">
        <Fa6RegChessPawn class="text-primary" />
        {{ seek.gameSettings.pieces }} Flat{{ seek.gameSettings.pieces !== 1 ? 's' : '' }}
      </div>
      <div v-if="isCapstonesSpecial" class="flex items-center gap-2 justify-start">
        <Fa6RegChessQueen class="text-primary" />
        {{ seek.gameSettings.capstones }} Capstone{{ seek.gameSettings.capstones !== 1 ? 's' : '' }}
      </div>
      <div v-if="isOpeningSpecial" class="flex items-center gap-2 justify-start">
        <LuPlay class="text-primary" />
        {{ openingNames[seek.gameSettings.opening] }}
      </div>
    </div>
  </Card>
</template>
