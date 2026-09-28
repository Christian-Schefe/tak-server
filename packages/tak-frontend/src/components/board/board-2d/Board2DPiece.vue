<script setup lang="ts">
import { board2dThemes } from '@/features/board2dThemes';
import { useSettingsStore } from '@/features/settings';
import type { TakUIPiece } from '@/tak-core/ui2d';
import { computed } from 'vue';

const props = defineProps<{
  piece: TakUIPiece;
  boardSize: number;
}>();

const transformData = computed(() => {
  const piece = props.piece;
  const effectiveHeight = piece.canBePicked ? piece.height - piece.buriedPieceCount : piece.height;
  const height = piece.isFloating ? effectiveHeight + 2 : effectiveHeight;

  const buriedLimit = 12;
  const buriedHeightOffset = Math.max(0, piece.buriedPieceCount - (buriedLimit - 1));

  const actualHeight = piece.canBePicked ? height : height - buriedHeightOffset;

  const zIndex =
    piece.zPriority !== null
      ? piece.zPriority + 50
      : piece.canBePicked
        ? actualHeight + 30
        : Math.max(-12, actualHeight + 12);

  const xTransform = piece.pos.x * 100 + (piece.canBePicked ? 0 : 35);
  const yTransform =
    (props.boardSize - 1 - piece.pos.y) * 100 - actualHeight * 7 + (piece.canBePicked ? 0 : 35);

  const hidden = !piece.canBePicked && piece.buriedPieceCount - height >= buriedLimit;

  return {
    zIndex,
    xTransform,
    yTransform,
    hidden,
    size: 100 / props.boardSize,
  };
});

const settingsStore = useSettingsStore();
const boardTheme = computed(
  () => board2dThemes[settingsStore.settings.boardTypeSettings['2d'].theme],
);

const styleData = computed(() => {
  const piece = props.piece;
  const pieceSize = 0.5;
  const wallWidthRatio = 2 / 5;
  const roundedPercent = boardTheme.value.pieces.rounded;
  const buriedSizeFactor = 0.25;
  const colors = piece.player === 'white' ? boardTheme.value.piece1 : boardTheme.value.piece2;
  return {
    width: piece.canBePicked ? pieceSize : pieceSize * buriedSizeFactor,
    height: piece.canBePicked
      ? piece.variant === 'standing'
        ? pieceSize * wallWidthRatio
        : pieceSize
      : pieceSize * buriedSizeFactor,
    borderRadius:
      piece.variant === 'standing'
        ? `${roundedPercent.toString()}% ${(roundedPercent / wallWidthRatio).toString()}%`
        : piece.variant === 'capstone'
          ? '100%'
          : `${roundedPercent.toString()}%`,
    rotation: piece.variant === 'standing' ? -45 : 0,
    outlineColor:
      piece.variant === 'capstone' && colors.capstoneOverride
        ? colors.capstoneOverride.border
        : colors.border,
    backgroundColor:
      piece.variant === 'capstone' && colors.capstoneOverride
        ? colors.capstoneOverride.background
        : colors.background,
    outlineWidth: boardTheme.value.pieces.border,
  };
});
</script>
<template>
  <div
    :style="{
      transform: `translate(${transformData.xTransform}%,${transformData.yTransform}%)`,
      width: `${transformData.size}%`,
      height: `${transformData.size}%`,
      zIndex: transformData.zIndex,
      transition: 'transform 0.2s ease, opacity 0.2s ease',
    }"
    class="absolute flex items-center justify-center"
  >
    <div
      class="inner-piece"
      :style="{
        transition: 'transform 0.2s ease, width 0.2s ease, height 0.2s ease, opacity 0.2s ease',
        width: `${styleData.width * 100}%`,
        height: `${styleData.height * 100}%`,
        opacity: transformData.hidden ? 0 : 1,
      }"
    >
      <div
        class="outline w-full h-full"
        :style="{
          borderBottomLeftRadius: styleData.borderRadius,
          borderBottomRightRadius: styleData.borderRadius,
          borderTopLeftRadius: styleData.borderRadius,
          borderTopRightRadius: styleData.borderRadius,
          transform: `rotate(${styleData.rotation}deg)`,
          backgroundColor: styleData.backgroundColor,
          outlineColor: styleData.outlineColor,
          outlineWidth: styleData.outlineWidth,
          transition: 'transform 0.2s ease',
        }"
      ></div>
    </div>
  </div>
</template>
<style lang="css" scoped>
.v-enter-from,
.v-leave-to {
  opacity: 0;
}
.v-enter-from .inner-piece,
.v-leave-to .inner-piece {
  transform: scale(0.8);
}
</style>
