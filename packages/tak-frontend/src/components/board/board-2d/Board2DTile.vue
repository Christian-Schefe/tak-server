<script setup lang="ts">
import { useSettingsStore } from '@/features/settings';
import type { TakGameResult, TakPos } from '@/tak-core';
import type { TakUITile } from '@/tak-core/ui';
import { board2dThemes } from '@/features/board2dThemes';
import Color from 'colorjs.io';
import { computed } from 'vue';

defineEmits<{
  (e: 'click'): void;
}>();

const props = defineProps<{
  pos: TakPos;
  tile: TakUITile;
  boardSize: number;
  plyIndex: number | null;
  gameResult: TakGameResult | null;
  interactive: boolean;
}>();

const settingsStore = useSettingsStore();
const boardTheme = computed(
  () => board2dThemes[settingsStore.settings.boardTypeSettings['2d'].theme],
);

const colorIndex = computed(() => {
  const { x, y } = props.pos;
  const isEven = (x + y) % 2 === 0;
  const size = props.boardSize;
  const ringCount = Math.ceil(size / 2);
  const ringIndex = Math.min(x, y, size - 1 - x, size - 1 - y) / (ringCount - 1);
  const themeParams = boardTheme.value;
  switch (themeParams.board.tiling) {
    case 'checkerboard':
      return isEven ? 0 : 1;
    case 'rings':
      return ringIndex;
    case 'linear':
      return (x + y) / (2 * (size - 1));
    case 'random': {
      const seed = (x * 73856093) ^ (y * 19349663);
      const rand = ((seed % 1000) + 1000) % 1000;
      return rand / 1000;
    }
    default:
      return 0;
  }
});

const specialHighlight = computed(() => {
  const plyIndex = props.plyIndex;
  const gameResult = props.gameResult;
  const { x, y } = props.pos;
  const isRoad =
    plyIndex === null &&
    gameResult?.type === 'win' &&
    gameResult.reason === 'road' &&
    gameResult.road?.some((coord) => coord.x === x && coord.y === y) === true;

  const isFlatWin =
    plyIndex === null &&
    gameResult?.type === 'win' &&
    gameResult.reason === 'flats' &&
    gameResult.flats?.some((coord) => coord.x === x && coord.y === y) === true;

  return isRoad || isFlatWin;
});

const isHover = computed(() => props.interactive && props.tile.hoverable);

const bgColor = computed(() => {
  return new Color(boardTheme.value.board1)
    .mix(new Color(boardTheme.value.board2), colorIndex.value)
    .toString();
});
const opacityTransition = 'opacity 200ms ease-in-out';
</script>

<template>
  <div
    class="relative flex items-center justify-center h-full w-full"
    :style="{
      'container-type': 'inline-size',
    }"
    @click="
      () => {
        if (props.interactive) $emit('click');
      }
    "
  >
    <div
      class="absolute inset-0 flex items-center justify-center"
      :style="{
        backgroundColor: boardTheme.tileSpecial?.hideBackground !== true ? bgColor : undefined,
        margin: boardTheme.board.spacing,
        borderRadius: boardTheme.board.rounded,
      }"
    >
      <div
        v-if="boardTheme.tileSpecial"
        class="rounded-full"
        :style="{
          backgroundColor: boardTheme.tileSpecial.color,
          width: boardTheme.tileSpecial.size,
          height: boardTheme.tileSpecial.size,
          borderRadius: boardTheme.tileSpecial.rounded,
          transform: boardTheme.tileSpecial.transform,
          outlineColor: boardTheme.tileSpecial.borderColor,
          outlineWidth: boardTheme.tileSpecial.border,
          outlineStyle: 'solid',
        }"
      ></div>
    </div>

    <div
      class="absolute inset-0"
      :style="{
        backgroundColor: boardTheme.highlight,
        opacity: tile.lastAction ? 1 : 0,
        transition: opacityTransition,
        margin: boardTheme.board.spacing,
        borderRadius: boardTheme.board.rounded,
      }"
    ></div>

    <div
      class="absolute inset-0"
      :style="{
        backgroundColor: boardTheme.hover,
        opacity: tile.selectable ? 0.8 : 0,
        transition: opacityTransition,
        margin: boardTheme.board.spacing,
        borderRadius: boardTheme.board.rounded,
      }"
    ></div>

    <div
      class="absolute inset-0"
      :class="{
        'opacity-0': !specialHighlight,
        'opacity-100': specialHighlight,
        'hover:opacity-100': isHover,
      }"
      :style="{
        backgroundColor: boardTheme.hover,
        transition: opacityTransition,
        margin: boardTheme.board.spacing,
        borderRadius: boardTheme.board.rounded,
      }"
    ></div>

    <div
      v-if="settingsStore.settings.boardTypeSettings['2d'].axisLabels && pos.y === 0"
      class="flex absolute right-1 bottom-0 justify-end items-end font-mono font-bold opacity-70"
      :style="{
        color: boardTheme.text,
        fontSize: `${settingsStore.settings.boardTypeSettings['2d'].axisLabelSize}cqw`,
        lineHeight: 1,
      }"
    >
      {{ String.fromCharCode('A'.charCodeAt(0) + pos.x).toUpperCase() }}
    </div>

    <div
      v-if="settingsStore.settings.boardTypeSettings['2d'].axisLabels && pos.x === 0"
      class="flex absolute left-1 top-0 justify-end items-end font-mono font-bold opacity-70"
      :style="{
        color: boardTheme.text,
        fontSize: `${settingsStore.settings.boardTypeSettings['2d'].axisLabelSize}cqw`,
        lineHeight: 1,
      }"
    >
      {{ pos.y + 1 }}
    </div>
  </div>
</template>
