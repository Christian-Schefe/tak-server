<script setup lang="ts">
import { useSettingsStore } from '@/features/settings.ts';
import type { TakAction, TakBaseGame, TakPlayer } from '@/tak-core';
import Board2D from './board/board-2d/Board2D.vue';
import Board3D from './board/board-3d/Board3D.vue';
import BoardNinja from './board/board-ninja/BoardNinja.vue';

export type GameMode =
  | { type: 'local' }
  | { type: 'online'; localPlayer: TakPlayer }
  | { type: 'spectator' };

defineEmits<{
  (e: 'action', action: TakAction): void;
}>();

const settingsStore = useSettingsStore();

const props = defineProps<{
  game: TakBaseGame;
  plyIndex: number | null;
  mode: GameMode;
}>();
</script>
<template>
  <div class="w-full h-full flex items-stretch flex-col xl:flex-row gap-2 p-2 xl:gap-4 xl:p-4">
    <div class="w-full xl:w-0 h-0 xl:h-full grow">
      <Board2D
        v-if="settingsStore.settings.boardType === '2d'"
        :game="props.game"
        :ply-index="props.plyIndex"
        :mode="props.mode"
        @action="$emit('action', $event)"
      ></Board2D>
      <Board3D
        v-else-if="settingsStore.settings.boardType === '3d'"
        :game="props.game"
        :ply-index="props.plyIndex"
        :mode="props.mode"
        @action="$emit('action', $event)"
      ></Board3D>
      <BoardNinja
        v-else
        :game="props.game"
        :ply-index="props.plyIndex"
        :mode="props.mode"
        @action="$emit('action', $event)"
      >
      </BoardNinja>
    </div>
    <div class="w-full h-20 xl:hidden bg-content border border-surface rounded-md flex">
      <slot name="mobile"></slot>
    </div>
    <div
      class="max-xl:hidden w-full xl:w-140 bg-content border border-surface rounded-md flex flex-col overflow-hidden xl:overflow-y-auto"
    >
      <slot name="desktop"></slot>
    </div>
  </div>
</template>
