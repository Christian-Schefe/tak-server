<script setup lang="ts">
import type { GameMode } from '@/components/Game.vue';
import { useSettingsStore } from '@/features/settings';
import {
  playerOpponent,
  type TakBaseGame,
  type TakPlayer,
  type TakPos,
  type TakVariant,
} from '@/tak-core';
import type { TakUI3DPiece } from '@/tak-core/ui3d';
import { type BoardPreset, type PiecePreset } from '@/features/board3dResources';
import { useLoop, type TresInstance } from '@tresjs/core';
import { Euler, MathUtils, Object3D, Quaternion, Vector3 } from 'three';
import { computed, shallowRef, watch } from 'vue';
import { SkeletonUtils, type GLTF } from 'three-stdlib';

interface LayoutData {
  pos: TakPos;
  player: TakPlayer;
  variant: TakVariant;
  height: number;
  isFloating: boolean;
  inReserve: boolean;
  effectivePlayer: TakPlayer;
}

const emit = defineEmits<{
  (e: 'clickReserve', player: TakPlayer, variant: 'flat' | 'capstone'): void;
}>();

const props = defineProps<{
  piece: TakUI3DPiece;
  game: TakBaseGame;
  boardSize: number;
  mode: GameMode;
  currentVariant: TakVariant | null;
  piecePreset: PiecePreset | undefined;
  boardPreset: BoardPreset | undefined;
  gltf: GLTF | null;
}>();

const layoutData = computed<LayoutData>(() => {
  const pieceData = props.piece;
  const game = props.game;
  const mode = props.mode;
  const currentVariant = props.currentVariant;
  if (pieceData.type === 'board') {
    const data: LayoutData = {
      pos: pieceData.pos,
      player: pieceData.id.player,
      variant: pieceData.variant,
      height: pieceData.height,
      isFloating: pieceData.isFloating,
      inReserve: false,
      effectivePlayer: pieceData.id.player,
    };
    return data;
  }
  const player = pieceData.id.player;
  const variant = pieceData.id.type;

  const reversedKindIndex =
    game.settings.reserve[variant === 'capstone' ? 'capstones' : 'pieces'] -
    1 -
    pieceData.id.kindIndex;

  const isFirstFlat = variant === 'flat' && reversedKindIndex === game.settings.reserve.pieces - 1;

  const effectivePlayer = game.actionHistory.length < 2 ? playerOpponent(player) : player;
  const isFloating =
    pieceData.isTopOfKind &&
    ((variant === 'capstone' && currentVariant === 'capstone') ||
      (variant === 'flat' && (currentVariant === 'flat' || currentVariant === 'standing'))) &&
    game.isOngoing() &&
    ((mode.type === 'online' && mode.localPlayer === effectivePlayer) ||
      (mode.type === 'local' && game.currentPlayer === effectivePlayer));
  const actualVariant = isFloating && currentVariant === 'standing' ? 'standing' : variant;

  const boardSize = game.settings.boardSize;
  const reserve = game.settings.reserve;

  const pieceStackSlots =
    variant === 'capstone' ? reserve.capstones : Math.max(2, boardSize - reserve.capstones);
  const piecesPerStack = Math.ceil(
    (variant === 'capstone' ? reserve.capstones : reserve.pieces) / pieceStackSlots,
  );
  const stack = pieceStackSlots - 1 - Math.floor(reversedKindIndex / piecesPerStack);
  const height = reversedKindIndex % piecesPerStack;
  const layoutData: LayoutData = {
    height,
    isFloating,
    player,
    pos: {
      x: (player === 'white') !== isFirstFlat ? -1.5 : boardSize + 0.5,
      y: stack + (variant === 'capstone' ? Math.max(boardSize - reserve.capstones, 2) : 0),
    },
    variant: actualVariant,
    inReserve: true,
    effectivePlayer: isFirstFlat ? playerOpponent(player) : player,
  };
  return layoutData;
});

const settingsStore = useSettingsStore();

const presetModel = computed(() => {
  const data = layoutData.value;
  return data.player === 'white'
    ? data.variant === 'capstone'
      ? props.piecePreset?.whiteCapstoneModel
      : props.piecePreset?.whitePieceModel
    : data.variant === 'capstone'
      ? props.piecePreset?.blackCapstoneModel
      : props.piecePreset?.blackPieceModel;
});

const pieceScale = computed(() => {
  return Math.max(Math.min(settingsStore.settings.boardTypeSettings['3d'].pieceScale, 1), 0.5);
});

const visualScale = computed(() => {
  const scale = pieceScale.value;
  const model = presetModel.value;
  return scale * (model?.scale ?? 1);
});

const positionOffset = computed(() => {
  const data = layoutData.value;
  const scale = pieceScale.value;
  const model = presetModel.value;

  const isStacked = data.height > 0;

  const baseOffsetVal =
    data.variant === 'standing' && model?.standingOffset ? model.standingOffset : model?.offset;

  const baseOffset = baseOffsetVal
    ? new Vector3(...baseOffsetVal).multiplyScalar(scale * (model?.scale ?? 1))
    : new Vector3(0, 0, 0);

  if (isStacked) {
    const baseStackedOffsetVal =
      data.variant === 'standing' && model?.stackedStandingOffset
        ? model.stackedStandingOffset
        : model?.stackedOffset;
    return baseOffset.add(
      baseStackedOffsetVal
        ? new Vector3(...baseStackedOffsetVal).multiplyScalar(scale * (model?.scale ?? 1))
        : new Vector3(0, 0, 0),
    );
  } else {
    return baseOffset;
  }
});

const targetPos = computed(() => {
  const data = layoutData.value;
  const pieceHeight =
    (props.piecePreset?.pieceHeight ?? 0) * pieceScale.value * (presetModel.value?.scale ?? 1);
  let height = (data.height + (data.isFloating ? 2 : 0)) * pieceHeight;
  if (!data.inReserve) height += props.boardPreset?.height ?? 0;
  return new Vector3(
    data.pos.x + 0.5 - props.boardSize / 2,
    height,
    -(data.pos.y + 0.5 - props.boardSize / 2),
  ).add(positionOffset.value);
});

const pieceRef = shallowRef<TresInstance | null>(null);

const targetRotation = computed(() => {
  const data = layoutData.value;
  const model = presetModel.value;
  if (data.variant === 'standing') {
    const standingRotation = model?.standingRotation ?? [
      0,
      45 * (data.player === 'white' ? 1 : -1),
      90,
    ];
    const radiansRotation = standingRotation.map((angle) => MathUtils.degToRad(angle));
    return new Quaternion().setFromEuler(new Euler(...radiansRotation));
  } else {
    return new Quaternion().setFromEuler(new Euler(0, 0, 0));
  }
});
const { onBeforeRender } = useLoop();
onBeforeRender((state) => {
  if (!pieceRef.value) return;
  const currentPos = pieceRef.value.position as Vector3;
  const currentQuat = pieceRef.value.quaternion as Quaternion;
  const targetDist = targetPos.value
    .clone()
    .setComponent(1, 0)
    .sub(currentPos.clone().setComponent(1, 0))
    .length();
  const newTargetPos = targetPos.value
    .clone()
    .addScaledVector(new Vector3(0, 1, 0), targetDist * 0.5);

  const actualDist = newTargetPos.clone().sub(currentPos.clone()).length();
  const lerpFactor = 0.2 * state.delta * 60;
  const moveLerpFactor = lerpFactor * MathUtils.lerp(2, 0.5, MathUtils.clamp(actualDist / 3, 0, 1));

  currentPos.copy(currentPos.clone().lerp(newTargetPos, moveLerpFactor));
  currentQuat.copy(currentQuat.clone().slerp(targetRotation.value, moveLerpFactor));
});

function onClick() {
  if (layoutData.value.inReserve) {
    emit(
      'clickReserve',
      layoutData.value.effectivePlayer,
      layoutData.value.variant === 'capstone' ? 'capstone' : 'flat',
    );
  }
}
watch(
  [pieceRef, () => layoutData.value.inReserve],
  ([piece, inReserve]) => {
    if (!piece) return;
    const hasEventListener: boolean = piece.hasEventListener('click', onClick);
    if (inReserve && !hasEventListener) {
      piece.addEventListener('click', onClick);
    }
    if (!inReserve && hasEventListener) {
      piece.removeEventListener('click', onClick);
    }
  },
  { immediate: true },
);

const sceneClone = computed<{ original: GLTF; clone: Object3D } | null>((oldClone) => {
  if (!props.gltf) return null;
  if (oldClone && oldClone.original === props.gltf) {
    return oldClone;
  }
  const clone: Object3D = SkeletonUtils.clone(props.gltf.scene);
  return { original: props.gltf, clone };
});
</script>

<template>
  <TresGroup ref="pieceRef" :scale="visualScale" @click="onClick">
    <primitive v-if="sceneClone" :object="sceneClone.clone"></primitive>
  </TresGroup>
</template>
