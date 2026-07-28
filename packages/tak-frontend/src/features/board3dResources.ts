import type { TakPlayer } from '@/tak-core';
import { useFetch } from '@/utils/fetch';
import { useQuery } from '@tanstack/vue-query';
import { useGLTF, useTexture } from '@tresjs/cientos';
import { SRGBColorSpace } from 'three';
import { computed, watch } from 'vue';
import { toValue, type MaybeRefOrGetter } from 'vue';
import { z } from 'zod';

export const board3dPiecePresets = [
  { name: 'Basic', id: 'basic' },
  { name: 'Bevel', id: 'bevel' },
] as const;
export type Board3DPiecePreset = (typeof board3dPiecePresets)[number]['id'];

export const board3dTablePresets = [{ name: 'Basic', id: 'basic' }] as const;
export type Board3DTablePreset = (typeof board3dTablePresets)[number]['id'];

export const board3dBoardPresets = [{ name: 'Basic', id: 'basic' }] as const;
export type Board3DBoardPreset = (typeof board3dBoardPresets)[number]['id'];

export const board3dTilesPresets = [
  { name: 'Basic', id: 'basic' },
  { name: 'Nok', id: 'nok' },
] as const;
export type Board3DTilesPreset = (typeof board3dTilesPresets)[number]['id'];

const pieceModel = z.object({
  fileName: z.string(),
  scale: z.number().default(1),
  offset: z.array(z.number()).length(3).optional(),
  standingOffset: z.array(z.number()).length(3).optional(),
  stackedOffset: z.array(z.number()).length(3).optional(),
  stackedStandingOffset: z.array(z.number()).length(3).optional(),
  standingRotation: z.array(z.number()).length(3).optional(),
});

const piecePreset = z.object({
  whitePieceModel: pieceModel,
  blackPieceModel: pieceModel,
  whiteCapstoneModel: pieceModel,
  blackCapstoneModel: pieceModel,

  pieceHeight: z.number(),
});

const tablePreset = z.object({
  fileName: z.string(),
  offset: z.array(z.number()).length(3).optional(),
  rotation: z.array(z.number()).length(3).optional(),
});

const boardModel = z.object({
  fileName: z.string(),
});

const boardPreset = z.object({
  models: z.object({
    3: boardModel,
    4: boardModel,
    5: boardModel,
    6: boardModel,
    7: boardModel,
    8: boardModel,
  }),
  offset: z.array(z.number()).length(3).optional(),
  height: z.number().optional(),
});

export type PiecePreset = z.infer<typeof piecePreset>;
export type TablePreset = z.infer<typeof tablePreset>;
export type BoardPreset = z.infer<typeof boardPreset>;

export function usePiecePreset(presetName: MaybeRefOrGetter<Board3DPiecePreset>) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['piecePreset', presetName],
    queryFn: async () => {
      const presetStr = toValue(presetName);
      return await fetchTyped(piecePreset, `/board-3d/piece/${presetStr}/piece.json`);
    },
  });
}

export function useTablePreset(presetName: MaybeRefOrGetter<Board3DTablePreset>) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['tablePreset', presetName],
    queryFn: async () => {
      const presetStr = toValue(presetName);
      return await fetchTyped(tablePreset, `/board-3d/table/${presetStr}/table.json`);
    },
  });
}

export function useBoardPreset(presetName: MaybeRefOrGetter<Board3DBoardPreset>) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['boardPreset', presetName],
    queryFn: async () => {
      const presetStr = toValue(presetName);
      return await fetchTyped(boardPreset, `/board-3d/board/${presetStr}/board.json`);
    },
  });
}

const defaultPiecePreset: PiecePreset = {
  whitePieceModel: {
    fileName: 'piece_white.glb',
    offset: [0, 0.125, 0],
    standingOffset: [0, 0.4, 0],
    scale: 1,
  },
  blackPieceModel: {
    fileName: 'piece_black.glb',
    offset: [0, 0.125, 0],
    standingOffset: [0, 0.4, 0],
    standingRotation: [0, -45, -90],
    scale: 1,
  },
  whiteCapstoneModel: {
    fileName: 'capstone_white.glb',
    offset: [0, 0.4, 0],
    scale: 1,
  },
  blackCapstoneModel: {
    fileName: 'capstone_black.glb',
    offset: [0, 0.4, 0],
    scale: 1,
  },
  pieceHeight: 0.25,
};

const defaultTablePreset: TablePreset = {
  fileName: 'table.glb',
  offset: [0, -0.6, 0],
  rotation: [0, 90, 0],
};

const defaultBoardPreset: BoardPreset = {
  models: {
    '3': { fileName: 'board_3x3.glb' },
    '4': { fileName: 'board_4x4.glb' },
    '5': { fileName: 'board_5x5.glb' },
    '6': { fileName: 'board_6x6.glb' },
    '7': { fileName: 'board_7x7.glb' },
    '8': { fileName: 'board_8x8.glb' },
  },
  offset: [0, -0.11, 0],
  height: 0.2,
};

export function getPieceModelPath(
  presetName: Board3DPiecePreset,
  preset: PiecePreset | undefined,
  pieceType: 'flat' | 'capstone',
  player: TakPlayer,
) {
  const effectivePreset = preset ?? defaultPiecePreset;
  const effectivePresetName = preset ? presetName : 'basic';
  const modelPath =
    pieceType === 'capstone'
      ? player === 'white'
        ? effectivePreset.whiteCapstoneModel.fileName
        : effectivePreset.blackCapstoneModel.fileName
      : player === 'white'
        ? effectivePreset.whitePieceModel.fileName
        : effectivePreset.blackPieceModel.fileName;
  return `/board-3d/piece/${effectivePresetName}/${modelPath}`;
}

export function getBoardTilesTexturePath(presetName: Board3DTilesPreset, boardSize: number) {
  return `/board-3d/tiles/${presetName}/board_${boardSize.toString()}x${boardSize.toString()}.png`;
}

export function getBoardModelPath(
  presetName: Board3DBoardPreset,
  preset: BoardPreset | undefined,
  boardSize: number,
) {
  const effectivePreset = preset ?? defaultBoardPreset;
  const effectivePresetName = preset ? presetName : 'basic';
  if (boardSize < 3 || boardSize > 8) {
    throw new Error(`Invalid board size: ${boardSize.toString()}. Must be between 3 and 8.`);
  }
  const modelPath =
    effectivePreset.models[boardSize as keyof typeof effectivePreset.models].fileName;
  return `/board-3d/board/${effectivePresetName}/${modelPath}`;
}

export function getTableModelPath(presetName: Board3DTablePreset, preset: TablePreset | undefined) {
  const effectivePreset = preset ?? defaultTablePreset;
  const effectivePresetName = preset ? presetName : 'basic';
  return `/board-3d/table/${effectivePresetName}/${effectivePreset.fileName}`;
}

export function useShadowGLTF(path: MaybeRefOrGetter<string>) {
  const { state } = useGLTF(computed(() => toValue(path)));
  watch(
    state,
    (newGltf) => {
      newGltf?.scene.traverse((child) => {
        child.castShadow = true;
        child.receiveShadow = true;
      });
    },
    { immediate: true },
  );
  return state;
}

export function useSRGBTexture(path: MaybeRefOrGetter<string>) {
  const { state: texture } = useTexture(computed(() => toValue(path)));
  watch(texture, () => {
    texture.value.colorSpace = SRGBColorSpace;
  });
  return texture;
}
