<script setup lang="ts">
import Game from '@/components/Game.vue';
import SettingsModal from '@/components/SettingsModal.vue';
import GameAnalysis from '@/components/side-panel/GameAnalysis.vue';
import GameSettingsConfigureDialog from '@/components/side-panel/GameSettingsConfigureDialog.vue';
import MoveHistory from '@/components/side-panel/MoveHistory.vue';
import { usePlayGameActionSound } from '@/features/sound';
import { TakBaseGame, type TakAction, type TakBaseGameSettings } from '@/tak-core';
import { Button, Card } from '@tak-ui-lib/components';
import { produce } from 'immer';
import { computed, ref, shallowRef, type ShallowRef } from 'vue';
import { LuSettings, LuUndo2, LuWrench } from 'vue-icons-plus/lu';

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

function onAnalysisAction(action: TakAction) {
  if (plyIndex.value !== null) {
    return;
  }
  onAction(action);
}

usePlayGameActionSound(game);

function onSettingsSubmit(settings: TakBaseGameSettings) {
  game.value = new TakBaseGame(settings);
  plyIndex.value = null;
}

const settingsVisible = ref(false);
const configureVisible = ref(false);

function onUndo() {
  game.value = produce(game.value, (game) => {
    game.undoAction();
  });
  plyIndex.value = null;
}

const canUndo = computed(() => game.value.canUndoAction());
</script>

<template>
  <Game :game="game" :ply-index="plyIndex" :mode="{ type: 'local' }" @action="onAction">
    <Card>
      <div class="w-full flex">
        <Button variant="text" severity="secondary" icon-only @click="settingsVisible = true">
          <LuSettings />
        </Button>
        <Button variant="text" severity="secondary" icon-only @click="configureVisible = true">
          <LuWrench />
        </Button>
        <Button variant="text" severity="secondary" :disabled="!canUndo" icon-only @click="onUndo">
          <LuUndo2 />
        </Button>
      </div>
    </Card>
    <SettingsModal v-model="settingsVisible"></SettingsModal>
    <GameSettingsConfigureDialog
      v-model="configureVisible"
      @apply="onSettingsSubmit"
    ></GameSettingsConfigureDialog>
    <GameAnalysis :game="game" :ply-index="plyIndex" @action="onAnalysisAction"></GameAnalysis>
    <MoveHistory
      :game="game"
      :ply-index="plyIndex"
      @update-ply-index="plyIndex = $event"
    ></MoveHistory>
  </Game>
</template>
