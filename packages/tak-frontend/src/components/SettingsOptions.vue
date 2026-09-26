<script setup lang="ts">
import { themes, type AppThemeId } from '@/features/appTheme';
import { board2dThemes, type Board2dThemeId } from '@/features/board2dThemes';
import { board3dPiecePresets, board3dTilesPresets } from '@/features/board3dResources';
import { ninja2dThemes } from '@/features/ninjaThemes';
import { useSettingsStore } from '@/features/settings';
import { Select, Slider, Toggle, type DarkMode } from '@tak-ui-lib/components';
import { computed } from 'vue';

const settingsStore = useSettingsStore();

const themeOptions: { label: string; value: AppThemeId }[] = Object.entries(themes).map(
  ([key, theme]) => ({
    label: theme.name,
    value: key as AppThemeId,
  }),
);
const themeModel = computed({
  get: () => settingsStore.settings.theme,
  set: (value) => {
    settingsStore.setTheme(value);
  },
});

const darkModeOptions: { value: DarkMode; label: string }[] = Object.entries({
  system: 'System',
  light: 'Light',
  dark: 'Dark',
}).map(([value, label]) => ({ value: value as DarkMode, label }));
const darkModeModel = computed({
  get: () => settingsStore.settings.darkMode,
  set: (value) => {
    settingsStore.setDarkMode(value);
  },
});

const audioVolumeModel = computed<number>({
  get: () => settingsStore.settings.audioVolume,
  set: (value) => {
    if (typeof value !== 'number') return;
    settingsStore.settings.audioVolume = value;
  },
});

const boardOptions: { label: string; value: 'ninja' | '2d' | '3d' }[] = [
  { label: '2D', value: '2d' },
  { label: '3D', value: '3d' },
  { label: 'Ninja', value: 'ninja' },
];
const boardModel = computed({
  get: () => settingsStore.settings.boardType,
  set: (value) => {
    settingsStore.settings.boardType = value;
  },
});

const board2dThemeOptions: { label: string; value: Board2dThemeId }[] = Object.entries(
  board2dThemes,
).map(([id, theme]) => ({
  label: theme.name,
  value: id as Board2dThemeId,
}));
const board2dThemeModel = computed({
  get: () => settingsStore.settings.boardTypeSettings['2d'].theme,
  set: (value) => {
    settingsStore.settings.boardTypeSettings['2d'].theme = value;
  },
});

const board2dAxisLabelSize = computed<number>({
  get: () => settingsStore.settings.boardTypeSettings['2d'].axisLabelSize,
  set: (value) => {
    settingsStore.settings.boardTypeSettings['2d'].axisLabelSize = value;
  },
});

const board2dAxisLabels = computed({
  get: () => settingsStore.settings.boardTypeSettings['2d'].axisLabels,
  set: (value) => {
    settingsStore.settings.boardTypeSettings['2d'].axisLabels = value;
  },
});

const boardNinjaThemeOptions = ninja2dThemes.map((theme) => ({
  label: `${theme[0]?.toUpperCase() ?? ''}${theme.slice(1)}`,
  value: theme,
}));

const boardNinjaThemeModel = computed({
  get: () => settingsStore.settings.boardTypeSettings.ninja.colorTheme,
  set: (value) => {
    settingsStore.settings.boardTypeSettings.ninja.colorTheme = value;
  },
});

const boardNinjaAxisLabelOptions = [
  { label: 'None', value: 'none' },
  { label: 'Small', value: 'small' },
  { label: 'Large', value: 'normal' },
] as const;

const boardNinjaAxisLabels = computed({
  get: () => settingsStore.settings.boardTypeSettings.ninja.axisLabels,
  set: (value) => {
    settingsStore.settings.boardTypeSettings.ninja.axisLabels = value;
  },
});

const boardNinjaAnimateBoard = computed({
  get: () => settingsStore.settings.boardTypeSettings.ninja.animateBoard,
  set: (value) => {
    settingsStore.settings.boardTypeSettings.ninja.animateBoard = value;
  },
});

const board3dPiecePresetOptions = board3dPiecePresets.map((preset) => ({
  label: preset.name,
  value: preset.id,
}));

const board3dPiecePresetModel = computed({
  get: () => settingsStore.settings.boardTypeSettings['3d'].piecePreset,
  set: (value) => {
    settingsStore.settings.boardTypeSettings['3d'].piecePreset = value;
  },
});

const board3dTilesPresetOptions = board3dTilesPresets.map((preset) => ({
  label: preset.name,
  value: preset.id,
}));

const board3dTilesPresetModel = computed({
  get: () => settingsStore.settings.boardTypeSettings['3d'].tilesPreset,
  set: (value) => {
    settingsStore.settings.boardTypeSettings['3d'].tilesPreset = value;
  },
});

const board3dPieceScale = computed<number>({
  get: () => settingsStore.settings.boardTypeSettings['3d'].pieceScale,
  set: (value) => {
    settingsStore.settings.boardTypeSettings['3d'].pieceScale = value;
  },
});
</script>
<template>
  <div class="flex flex-col gap-4 w-full">
    <h2>General Settings</h2>
    <Select v-model="themeModel" :options="themeOptions" label="Theme" />
    <Select v-model="darkModeModel" :options="darkModeOptions" label="Color Scheme" />
    <div class="grid gap-4 items-center" :style="{ gridTemplateColumns: 'auto 1fr auto' }">
      <p>Audio Volume</p>
      <Slider v-model="audioVolumeModel" :min="0" :max="1" :step="0.01" />
      <p class="min-w-12 text-right">
        {{ typeof audioVolumeModel === 'number' ? (audioVolumeModel * 100).toFixed(0) : '' }}%
      </p>
    </div>
    <Select v-model="boardModel" :options="boardOptions" label="Board Type" />
    <h2>{{ { '2d': '2D Settings', '3d': '3D Settings', ninja: 'Ninja Settings' }[boardModel] }}</h2>
    <template v-if="boardModel === '2d'">
      <Select v-model="board2dThemeModel" :options="board2dThemeOptions" label="Theme"></Select>
      <div class="grid gap-2 items-center" :style="{ gridTemplateColumns: 'auto auto 1fr' }">
        <p>Axis Labels</p>
        <Toggle v-model="board2dAxisLabels" />
        <Slider v-model="board2dAxisLabelSize" :disabled="!board2dAxisLabels" />
      </div>
    </template>
    <template v-else-if="boardModel === 'ninja'">
      <Select
        v-model="boardNinjaThemeModel"
        :options="boardNinjaThemeOptions"
        label="Theme"
      ></Select>
      <Select
        v-model="boardNinjaAxisLabels"
        :options="boardNinjaAxisLabelOptions"
        label="Axis Labels"
      ></Select>
      <p>Animations</p>
      <Toggle v-model="boardNinjaAnimateBoard" />
    </template>
    <template v-else-if="boardModel === '3d'">
      <Select
        v-model="board3dPiecePresetModel"
        :options="board3dPiecePresetOptions"
        label="Piece Preset"
      ></Select>
      <Select
        v-model="board3dTilesPresetModel"
        :options="board3dTilesPresetOptions"
        label="Tiles Preset"
      ></Select>
      <div class="grid gap-4 items-center" :style="{ gridTemplateColumns: 'auto 1fr auto' }">
        <p>Piece Scale</p>
        <Slider
          v-model="board3dPieceScale"
          :disabled="!board3dPieceScale"
          :min="0.5"
          :max="1"
          :step="0.01"
        />
        <p class="min-w-12 text-right">
          {{ typeof board3dPieceScale === 'number' ? (board3dPieceScale * 100).toFixed(0) : '' }}%
        </p>
      </div>
    </template>
  </div>
</template>
