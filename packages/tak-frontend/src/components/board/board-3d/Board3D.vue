<script setup lang="ts">
import type { GameMode } from '@/components/Game.vue';
import { useSettingsStore } from '@/features/settings.ts';
import {
  baseGameSettingsEquals,
  type TakAction,
  type TakBaseGame,
  type TakPlayer,
  type TakPos,
  type TakVariant,
} from '@/tak-core';
import type { TakUITile } from '@/tak-core/ui';
import { TakGame3DUI, type TakUI3DPiece } from '@/tak-core/ui3d';
import {
  getBoardModelPath,
  getBoardTilesTexturePath,
  getPieceModelPath,
  getTableModelPath,
  useBoardPreset,
  usePiecePreset,
  useShadowGLTF,
  useSRGBTexture,
  useTablePreset,
} from '@/features/board3dResources.ts';
import { OrbitControls } from '@tresjs/cientos';
import { TresCanvas, type TresPointerEvent } from '@tresjs/core';
import { BloomPmndrs, EffectComposerPmndrs, FXAAPmndrs } from '@tresjs/post-processing';
import { produce } from 'immer';
import { MOUSE, PCFShadowMap, type DirectionalLight } from 'three';
import { computed, ref, shallowRef, watch, type ShallowRef } from 'vue';
import Board3DBoard from './Board3DBoard.vue';
import Board3DPiece from './Board3DPiece.vue';
import Board3DTable from './Board3DTable.vue';

const emit = defineEmits<{
  (e: 'action', action: TakAction): void;
}>();

const props = defineProps<{
  game: TakBaseGame;
  plyIndex: number | null;
  mode: GameMode;
}>();

function computeNewGame(
  oldGameUi: TakGame3DUI | undefined,
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
    : new TakGame3DUI(shownGame);
}

const gameUi = shallowRef<TakGame3DUI>(
  computeNewGame(undefined, props.game, props.plyIndex),
) as ShallowRef<TakGame3DUI>;
watch(
  () => [props.game, props.plyIndex] as const,
  ([newGame, newPlyIndex]) => {
    gameUi.value = computeNewGame(gameUi.value, newGame, newPlyIndex);
  },
);

const tilePositions = computed(() => {
  const tiles = gameUi.value.tiles;
  const gameSettings = gameUi.value.actualGame.settings;
  const tileData: { pos: TakPos; data: TakUITile; posVec: [number, number, number] }[] = [];
  for (let y = gameSettings.boardSize - 1; y >= 0; y--) {
    for (let x = 0; x < gameSettings.boardSize; x++) {
      const tile = tiles[y * gameSettings.boardSize + x];
      if (!tile) continue;
      tileData.push({
        pos: { x, y },
        data: tile,
        posVec: [
          x - gameSettings.boardSize / 2 + 0.5,
          boardPreset.value?.height ?? 0,
          -(y - gameSettings.boardSize / 2 + 0.5),
        ],
      });
    }
  }
  return tileData;
});

const pieceData = computed<TakUI3DPiece[]>(() => {
  return gameUi.value.pieces;
});

const settingsStore = useSettingsStore();

const { data: piecePreset } = usePiecePreset(
  () => settingsStore.settings.boardTypeSettings['3d'].piecePreset,
);

const { data: tablePreset } = useTablePreset(() => 'basic');

const { data: boardPreset } = useBoardPreset(() => 'basic');

const lightRef = ref<DirectionalLight | null>(null);

watch(lightRef, (light) => {
  if (light) {
    light.shadow.bias = -0.001;
    light.shadow.mapSize.width = 2048;
    light.shadow.mapSize.height = 2048;
    light.shadow.camera.far = 40;
    light.shadow.camera.left = -6;
    light.shadow.camera.right = 6;
    light.shadow.camera.top = 6;
    light.shadow.camera.bottom = -6;
  }
});

function onClickTile(event: TresPointerEvent) {
  event.stopPropagation();
  if (!areTilesInteractive.value) return;
  const pos = event.object.userData.pos as TakPos;
  const action = gameUi.value.tryPlaceOrAddToPartialAction(pos, currentVariant.value ?? 'flat');
  gameUi.value = produce(gameUi.value, (gameUi) => {
    if (action) {
      emit('action', action);
    } else {
      gameUi.updatePartialAction(pos);
    }
    return gameUi;
  });
  currentVariant.value = null;
}

function onClickPiece(player: TakPlayer, variant: 'flat' | 'capstone') {
  switch (props.mode.type) {
    case 'local':
      if (props.game.currentPlayer !== player) {
        return;
      }
      break;
    case 'online':
      if (props.mode.localPlayer !== player) {
        return;
      }
      break;
    default:
      return;
  }
  setCurrentVariant(variant === 'capstone');
}

const boardTextureUrl = computed(() => {
  const boardSize = gameUi.value.actualGame.settings.boardSize;
  return getBoardTilesTexturePath(
    settingsStore.settings.boardTypeSettings['3d'].tilesPreset,
    boardSize,
  );
});

const boardTexture = useSRGBTexture(boardTextureUrl);
const selectionTexture = useSRGBTexture('/board-3d/square_select.png');

const areTilesInteractive = computed(() => {
  return (
    ((props.mode.type === 'online' && props.game.currentPlayer === props.mode.localPlayer) ||
      props.mode.type === 'local') &&
    props.game.isOngoing() &&
    props.plyIndex === null
  );
});

const currentVariant = ref<TakVariant | null>(null);

function setCurrentVariant(isCapstone: boolean) {
  if (!areTilesInteractive.value) return;

  if (!canPlace.value) {
    currentVariant.value = null;
    return;
  }

  if (isCapstone && canPlace.value.capstone && currentVariant.value !== 'capstone') {
    currentVariant.value = 'capstone';
    return;
  } else if (isCapstone) {
    currentVariant.value = null;
    return;
  }

  if (currentVariant.value === 'flat' && canPlace.value.standing) {
    currentVariant.value = 'standing';
    return;
  }
  if (
    (currentVariant.value === null || currentVariant.value === 'capstone') &&
    canPlace.value.flat
  ) {
    currentVariant.value = 'flat';
    return;
  }
  currentVariant.value = null;
}

const canPlace = computed(() => {
  let player: TakPlayer;
  if (props.mode.type === 'local') {
    player = props.game.currentPlayer;
  } else if (props.mode.type === 'online') {
    player = props.mode.localPlayer;
  } else {
    return null;
  }
  const reserves = props.game.reserves[player];
  return {
    flat: reserves.pieces > 0,
    standing: reserves.pieces > 0 && props.game.actionHistory.length >= 2,
    capstone: reserves.capstones > 0 && props.game.actionHistory.length >= 2,
  };
});

watch([canPlace, areTilesInteractive], ([newCanPlace, newAreTilesInteractive]) => {
  function getNewVariant() {
    if (!newAreTilesInteractive) return null;
    const canPlace = newCanPlace;

    if (canPlace) {
      if (currentVariant.value === 'flat' && !canPlace.flat) {
        if (canPlace.standing) return 'standing';
        if (canPlace.capstone) return 'capstone';
      }
      if (currentVariant.value === 'standing' && !canPlace.standing) {
        if (canPlace.flat) return 'flat';
        if (canPlace.capstone) return 'capstone';
      }
      if (currentVariant.value === 'capstone' && !canPlace.capstone) {
        if (canPlace.flat) return 'flat';
        if (canPlace.standing) return 'standing';
      }
    }
    return null;
  }
  const newVariant = getNewVariant();
  if (newVariant !== currentVariant.value) {
    currentVariant.value = newVariant;
  }
});

const hoveredTile = ref<TakPos | null>(null);

function onLeaveTile(event: TresPointerEvent) {
  event.stopPropagation();
  const pos = event.object.userData.pos as TakPos;
  if (hoveredTile.value && hoveredTile.value.x === pos.x && hoveredTile.value.y === pos.y) {
    hoveredTile.value = null;
  }
}

const showHoverHighlight = computed(() => {
  if (!hoveredTile.value || !areTilesInteractive.value) return false;
  const index =
    hoveredTile.value.y * gameUi.value.actualGame.settings.boardSize + hoveredTile.value.x;
  const tile = gameUi.value.tiles[index];
  return tile?.hoverable ?? false;
});

const lastActionTiles = computed(() => {
  return tilePositions.value.filter((tile) => tile.data.lastAction).map((tile) => tile.pos);
});

const modelPathWhiteFlat = computed(() => {
  return getPieceModelPath(
    settingsStore.settings.boardTypeSettings['3d'].piecePreset,
    piecePreset.value,
    'flat',
    'white',
  );
});
const modelPathBlackFlat = computed(() => {
  return getPieceModelPath(
    settingsStore.settings.boardTypeSettings['3d'].piecePreset,
    piecePreset.value,
    'flat',
    'black',
  );
});
const modelPathWhiteCapstone = computed(() => {
  return getPieceModelPath(
    settingsStore.settings.boardTypeSettings['3d'].piecePreset,
    piecePreset.value,
    'capstone',
    'white',
  );
});
const modelPathBlackCapstone = computed(() => {
  return getPieceModelPath(
    settingsStore.settings.boardTypeSettings['3d'].piecePreset,
    piecePreset.value,
    'capstone',
    'black',
  );
});
const modelPathTable = computed(() => {
  return getTableModelPath('basic', tablePreset.value);
});
const modelPathBoard = computed(() => {
  return getBoardModelPath('basic', boardPreset.value, gameUi.value.actualGame.settings.boardSize);
});
const whiteFlatState = useShadowGLTF(modelPathWhiteFlat);
const blackFlatState = useShadowGLTF(modelPathBlackFlat);
const whiteCapstoneState = useShadowGLTF(modelPathWhiteCapstone);
const blackCapstoneState = useShadowGLTF(modelPathBlackCapstone);
const tableState = useShadowGLTF(modelPathTable);
const boardState = useShadowGLTF(modelPathBoard);
const gltfs = computed(() => {
  return {
    white: {
      flat: whiteFlatState.value,
      capstone: whiteCapstoneState.value,
    },
    black: {
      flat: blackFlatState.value,
      capstone: blackCapstoneState.value,
    },
  };
});
</script>

<template>
  <div class="w-full h-full min-h-[60vh] relative">
    <div :class="`absolute inset-0 rounded-md overflow-hidden transition-opacity`">
      <TresCanvas clear-color="#000000" :clear-alpha="0" shadows :shadow-map-type="PCFShadowMap">
        <TresGroup>
          <Board3DPiece
            v-for="piece in pieceData"
            :key="piece.index"
            :piece="piece"
            :board-size="gameUi.actualGame.settings.boardSize"
            :mode="mode"
            :game="gameUi.actualGame"
            :current-variant="currentVariant"
            :piece-preset="piecePreset"
            :board-preset="boardPreset"
            :gltf="gltfs[piece.id.player][piece.id.type]"
            @click-reserve="onClickPiece"
          ></Board3DPiece>
          <TresMesh
            v-for="(item, index) in tilePositions"
            :key="index"
            :position="item.posVec"
            :rotation="[-Math.PI / 2, 0, 0]"
            :visible="false"
            :user-data="{ pos: item.pos }"
            @pointerenter="hoveredTile = item.pos"
            @pointerleave="onLeaveTile"
            @click="onClickTile"
          >
            <TresPlaneGeometry :args="[1, 1]" />
            <TresMeshStandardMaterial />
          </TresMesh>
        </TresGroup>
        <Board3DTable :table-preset="tablePreset" :gltf="tableState" />
        <Board3DBoard :board-preset="boardPreset" :gltf="boardState" />
        <TresGroup>
          <TresMesh
            :position="[0, 0.01 + (boardPreset?.height ?? 0), 0]"
            :rotation="[-Math.PI / 2, 0, 0]"
            receive-shadow
          >
            <TresPlaneGeometry
              :args="[gameUi.actualGame.settings.boardSize, gameUi.actualGame.settings.boardSize]"
            />
            <TresMeshStandardMaterial :map="boardTexture" />
          </TresMesh>
          <TresMesh
            v-for="(tile, index) in lastActionTiles"
            :key="index"
            :position="[
              tile.x - gameUi.actualGame.settings.boardSize / 2 + 0.5,
              0.02 + (boardPreset?.height ?? 0),
              -(tile.y - gameUi.actualGame.settings.boardSize / 2 + 0.5),
            ]"
            :rotation="[-Math.PI / 2, 0, 0]"
          >
            <TresPlaneGeometry :args="[0.9, 0.9]" />
            <TresMeshStandardMaterial
              :color="'#5255E1'"
              :map="selectionTexture"
              transparent
              :opacity="0.8"
            />
          </TresMesh>
          <TresMesh
            v-if="hoveredTile && showHoverHighlight"
            :position="[
              hoveredTile.x - gameUi.actualGame.settings.boardSize / 2 + 0.5,
              0.02 + (boardPreset?.height ?? 0),
              -(hoveredTile.y - gameUi.actualGame.settings.boardSize / 2 + 0.5),
            ]"
            :rotation="[-Math.PI / 2, 0, 0]"
          >
            <TresPlaneGeometry :args="[0.9, 0.9]" />
            <TresMeshStandardMaterial :color="'#3B82F6'" :map="selectionTexture" transparent />
          </TresMesh>
        </TresGroup>
        <TresPerspectiveCamera :position="[0, 9, 15]" :look-at="[0, 0, 0]" />
        <TresAmbientLight :intensity="0.7" />
        <TresSpotLight
          :position="[5, 10, -10]"
          :intensity="1.0"
          :angle="0.5"
          :penumbra="1"
          :decay="0"
        />
        <TresDirectionalLight
          ref="lightRef"
          :cast-shadow="true"
          :position="[4, 8, 6]"
          :intensity="1.5"
        />
        <TresPointLight :position="[-10, 10, -10]" :intensity="1.0" :decay="0" />

        <OrbitControls
          :make-default="true"
          :enable-pan="false"
          :mouse-buttons="{
            LEFT: MOUSE.ROTATE,
            MIDDLE: undefined,
            RIGHT: MOUSE.ROTATE,
          }"
          :max-polar-angle="1.5"
        >
        </OrbitControls>
        <Suspense>
          <EffectComposerPmndrs>
            <FXAAPmndrs />
            <BloomPmndrs
              :intensity="1.0"
              :luminance-threshold="0.9"
              :luminance-smoothing="0.1"
              mipmap-blur
            />
          </EffectComposerPmndrs>
        </Suspense>
      </TresCanvas>
    </div>
  </div>
</template>
