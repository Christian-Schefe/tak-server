<script setup lang="ts">
import Game from '@/components/Game.vue';
import SettingsModal from '@/components/SettingsModal.vue';
import SidePanelAccordion from '@/components/side-panel/SidePanelAccordion.vue';
import SidePanelMobile from '@/components/side-panel/SidePanelMobile.vue';
import type { SidePanelSection } from '@/features/sidePanel';
import { usePlayGameActionSound } from '@/features/sound';
import { TakBaseGame, type TakAction, type TakBaseGameSettings } from '@/tak-core';
import { produce } from 'immer';
import Button from 'primevue/button';
import { computed, ref, shallowRef, type ShallowRef } from 'vue';
import { LuSettings, LuUndo2 } from 'vue-icons-plus/lu';

function createNewGame() {
  const game = new TakBaseGame({
    boardSize: 6,
    halfKomi: 4,
    reserve: {
      capstones: 1,
      pieces: 30,
    },
    opening: 'swap',
  });
  return game;
}

const game = shallowRef<TakBaseGame>(createNewGame()) as ShallowRef<TakBaseGame>;
const plyIndex = ref<number | null>(null);

function onAction(action: TakAction) {
  console.log('Action:', action);
  game.value = produce(game.value, (game) => {
    game.doAction(action);
  });
}

usePlayGameActionSound(game);

function onSettingsSubmit(settings: TakBaseGameSettings) {
  game.value = new TakBaseGame(settings);
  plyIndex.value = null;
}

const settingsVisible = ref(false);

function onUndo() {
  game.value = produce(game.value, (game) => {
    game.undoAction();
  });
  plyIndex.value = null;
}

const canUndo = computed(() => game.value.canUndoAction());

const sidePanelSections: SidePanelSection[] = [
  { type: 'configure' },
  { type: 'analysis' },
  { type: 'game_info' },
];
</script>

<template>
  <Game :game="game" :ply-index="plyIndex" :mode="{ type: 'local' }" @action="onAction">
    <template #desktop>
      <div class="w-full p-2 border-b border-surface flex">
        <Button variant="text" severity="secondary" @click="settingsVisible = true">
          <template #icon><LuSettings></LuSettings></template>
        </Button>
        <Button variant="text" severity="secondary" :disabled="!canUndo" @click="onUndo">
          <template #icon><LuUndo2></LuUndo2></template>
        </Button>
        <SettingsModal v-model="settingsVisible"></SettingsModal>
      </div>
      <SidePanelAccordion
        v-model:ply-index="plyIndex"
        :game="game"
        :sections="sidePanelSections"
        @settings-submit="onSettingsSubmit"
        @analysis-action="onAction"
      ></SidePanelAccordion>
    </template>
    <template #mobile>
      <SidePanelMobile
        v-model:ply-index="plyIndex"
        :game="game"
        :sections="sidePanelSections"
        @settings-submit="onSettingsSubmit"
        @analysis-action="onAction"
      />
    </template>
  </Game>
</template>
