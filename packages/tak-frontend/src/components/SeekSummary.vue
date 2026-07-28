<script setup lang="ts">
import type { SeekInfo } from '@/api/seek.ts';
import { timeControlToString } from '@/utils/time.ts';
import Button from 'primevue/button';
import Tag from 'primevue/tag';
import { Fa6ChessBoard } from 'vue-icons-plus/fa6';
import { LuClock, LuContrast, LuScale, LuSwords, LuTrash } from 'vue-icons-plus/lu';
import GameSettingsPopover from './GameSettingsPopover.vue';
import PlayerLabel from './PlayerLabel.vue';

defineProps<{
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
</script>
<template>
  <div class="grow flex flex-col gap-2 p-2 bg-content rounded-md">
    <div class="flex gap-2">
      <div class="flex flex-col gap-2 justify-center">
        <PlayerLabel :pid="seek.creatorId" type="player"></PlayerLabel>
      </div>
      <Tag v-if="!seek.isRated" severity="warn">Unrated</Tag>
      <div class="grow" />

      <GameSettingsPopover :settings="seek.gameSettings" />
      <Button class="w-8! h-8! p-1!" severity="secondary" @click="$emit('click')">
        <template #icon>
          <LuTrash v-if="action === 'delete'" />
          <LuSwords v-else />
        </template>
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
    </div>
  </div>
</template>
