<script setup lang="ts">
import type { GameMode } from '@/components/Game.vue';
import {
  baseGameSettingsEquals,
  type TakAction,
  type TakBaseGame,
  type TakPlayer,
  type TakPos,
  type TakVariant,
} from '@/tak-core';
import { TakGameUI, type TakUIPiece, type TakUITile } from '@/tak-core/ui';
import { produce } from 'immer';
import Button from 'primevue/button';
import { computed, ref, shallowRef, watch, type ShallowRef } from 'vue';
import Board2DPiece from './Board2DPiece.vue';
import Board2DTile from './Board2DTile.vue';
import { useSettingsStore } from '@/features/settings.ts';
import { board2dThemes } from '@/features/board2dThemes.ts';

const emit = defineEmits<{
  (e: 'action', action: TakAction): void;
}>();

const props = defineProps<{
  game: TakBaseGame;
  plyIndex: number | null;
  mode: GameMode;
}>();

function computeNewGame(
  oldGameUi: TakGameUI | undefined,
  newGame: TakBaseGame,
  newPlyIndex: number | null,
) {
  let shownGame = newGame;
  if (newPlyIndex !== null) {
    shownGame = shownGame.clone();
    shownGame.trimToPlyIndex(newPlyIndex);
  }
  return oldGameUi && baseGameSettingsEquals(oldGameUi.actualGame.settings, shownGame.settings)
    ? produce(oldGameUi, (gameUi) => {
        gameUi.updateGame(shownGame);
        return gameUi;
      })
    : new TakGameUI(shownGame);
}

const gameUi = shallowRef<TakGameUI>(
  computeNewGame(undefined, props.game, props.plyIndex),
) as ShallowRef<TakGameUI>;
watch(
  () => [props.game, props.plyIndex] as const,
  ([newGame, newPlyIndex]) => {
    gameUi.value = computeNewGame(gameUi.value, newGame, newPlyIndex);
  },
);

const tilePositions = computed(() => {
  const tiles = gameUi.value.tiles;
  const gameSettings = gameUi.value.actualGame.settings;
  const tileData: { pos: TakPos; data: TakUITile }[] = [];
  for (let y = gameSettings.boardSize - 1; y >= 0; y--) {
    for (let x = 0; x < gameSettings.boardSize; x++) {
      const tile = tiles[y * gameSettings.boardSize + x];
      if (!tile) continue;
      tileData.push({ pos: { x, y }, data: tile });
    }
  }
  return tileData;
});

const pieceData = computed(() => {
  const pieces = gameUi.value.pieces;
  const pieceIds = Object.entries(pieces)
    .map(([id, data]) =>
      data
        ? {
            id,
            data,
          }
        : null,
    )
    .filter((piece): piece is { id: string; data: TakUIPiece } => piece !== null);
  pieceIds.sort((a, b) => a.id.localeCompare(b.id));
  return pieceIds;
});

const currentVariant = ref<TakVariant>('flat');

function onClickTile(pos: TakPos) {
  if (!areTilesInteractive.value) return;
  const action = gameUi.value.tryPlaceOrAddToPartialAction(pos, currentVariant.value);
  gameUi.value = produce(gameUi.value, (gameUi) => {
    if (action) {
      emit('action', action);
    } else {
      gameUi.updatePartialAction(pos);
    }
    return gameUi;
  });
}

const areTilesInteractive = computed(() => {
  return (
    ((props.mode.type === 'online' && props.game.currentPlayer === props.mode.localPlayer) ||
      props.mode.type === 'local') &&
    props.game.gameResult === null &&
    props.plyIndex === null
  );
});

const canPlace = computed(() => {
  let player: TakPlayer;
  if (props.mode.type === 'local') {
    player = props.game.currentPlayer;
  } else if (props.mode.type === 'online') {
    player = props.mode.localPlayer;
  } else {
    return null;
  }
  const isOngoing = props.game.isOngoing();
  const reserves = props.game.reserves[player];
  return {
    flat: isOngoing && reserves.pieces > 0,
    standing: isOngoing && reserves.pieces > 0 && props.game.actionHistory.length >= 2,
    capstone: isOngoing && reserves.capstones > 0 && props.game.actionHistory.length >= 2,
  };
});

watch([canPlace, currentVariant], ([newCanPlace, newVariant]) => {
  if (newCanPlace !== null && !newCanPlace[newVariant]) {
    if (newCanPlace.flat) {
      currentVariant.value = 'flat';
    } else if (newCanPlace.capstone) {
      currentVariant.value = 'capstone';
    }
  }
});

const settingsStore = useSettingsStore();
const boardTheme = computed(
  () => board2dThemes[settingsStore.settings.boardTypeSettings['2d'].theme],
);
</script>
<template>
  <div class="w-full h-full relative">
    <div class="absolute inset-0 m-auto max-w-full max-h-full touch-none aspect-4/5 xl:aspect-9/10">
      <div class="w-full h-[10%] xl:h-[5%]">
        <div class="w-full h-full grid grid-cols-2 gap-2 pb-2">
          <div
            class="bg-content border-surface rounded-md flex gap-4 items-center justify-center font-mono outline-primary"
            :class="gameUi.actualGame.currentPlayer === 'white' ? 'outline-2' : ''"
          >
            <span class="font-semibold">White</span> {{ game.reserves['white'].pieces }} /
            {{ game.reserves['white'].capstones }}
          </div>
          <div
            class="bg-content border-surface rounded-md flex gap-4 items-center justify-center font-mono outline-primary"
            :class="gameUi.actualGame.currentPlayer === 'black' ? 'outline-2' : ''"
          >
            <span class="font-semibold">Black</span> {{ game.reserves['black'].pieces }} /
            {{ game.reserves['black'].capstones }}
          </div>
        </div>
      </div>
      <div class="w-full h-[80%] xl:h-[90%] relative">
        <div
          class="w-full h-full grid overflow-hidden rounded-md"
          :style="{
            gridTemplateColumns: `repeat(${gameUi.actualGame.settings.boardSize}, minmax(0, 1fr))`,
            backgroundColor: boardTheme.background,
          }"
        >
          <Board2DTile
            v-for="(tile, index) in tilePositions"
            :key="index"
            :tile="tile.data"
            :pos="tile.pos"
            :board-size="gameUi.actualGame.settings.boardSize"
            :ply-index="plyIndex"
            :interactive="areTilesInteractive"
            :game-result="gameUi.actualGame.gameResult"
            @click="onClickTile(tile.pos)"
          ></Board2DTile>
        </div>
        <div class="absolute inset-0 pointer-events-none">
          <Board2DPiece
            v-for="piece in pieceData"
            :key="piece.id"
            :piece="piece.data"
            :board-size="gameUi.actualGame.settings.boardSize"
          ></Board2DPiece>
        </div>
      </div>
      <div class="w-full h-[10%] xl:h-[5%]">
        <div class="w-full h-full grid grid-cols-3 gap-2 pt-2 font-mono">
          <Button
            fluid
            :severity="currentVariant === 'flat' ? undefined : 'secondary'"
            :disabled="canPlace === null || !canPlace.flat"
            @click="currentVariant = 'flat'"
          >
            Flat
          </Button>
          <Button
            fluid
            :severity="currentVariant === 'standing' ? undefined : 'secondary'"
            :disabled="canPlace === null || !canPlace.standing"
            @click="currentVariant = 'standing'"
          >
            Wall
          </Button>
          <Button
            fluid
            :severity="currentVariant === 'capstone' ? undefined : 'secondary'"
            :disabled="canPlace === null || !canPlace.capstone"
            @click="currentVariant = 'capstone'"
          >
            Capstone
          </Button>
        </div>
      </div>
    </div>
  </div>
</template>
