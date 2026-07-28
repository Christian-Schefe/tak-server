<script setup lang="ts">
import { useSettingsStore } from '@/features/settings';
import { themes } from '@/features/appTheme';
import { board2dThemes } from '@/features/board2dThemes';
import { board3dPiecePresets, board3dTilesPresets } from '@/features/board3dResources';
import { ninja2dThemes } from '@/features/ninjaThemes';
import Divider from 'primevue/divider';
import IftaLabel from 'primevue/iftalabel';
import Select from 'primevue/select';
import Slider from 'primevue/slider';
import ToggleButton from 'primevue/togglebutton';
import { computed } from 'vue';

const settingsStore = useSettingsStore();

const themeOptions = Object.entries(themes).map(([key, theme]) => ({
  label: theme.name,
  value: key,
}));
const themeModel = computed({
  get: () => settingsStore.settings.theme,
  set: (value) => {
    settingsStore.setTheme(value);
  },
});

const darkModeOptions = Object.entries({
  system: 'System',
  light: 'Light',
  dark: 'Dark',
}).map(([value, label]) => ({ value, label }));
const darkModeModel = computed({
  get: () => settingsStore.settings.darkMode,
  set: (value) => {
    settingsStore.setDarkMode(value);
  },
});

const audioVolumeModel = computed<number | number[]>({
  get: () => settingsStore.settings.audioVolume,
  set: (value) => {
    if (typeof value !== 'number') return;
    settingsStore.settings.audioVolume = value;
  },
});

const boardOptions = [
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

const board2dThemeOptions = Object.entries(board2dThemes).map(([id, theme]) => ({
  label: theme.name,
  value: id,
}));
const board2dThemeModel = computed({
  get: () => settingsStore.settings.boardTypeSettings['2d'].theme,
  set: (value) => {
    settingsStore.settings.boardTypeSettings['2d'].theme = value;
  },
});

const board2dAxisLabelSize = computed<number | number[]>({
  get: () => settingsStore.settings.boardTypeSettings['2d'].axisLabelSize,
  set: (value) => {
    if (typeof value !== 'number') return;
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
];

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

const board3dPieceScale = computed<number | number[]>({
  get: () => settingsStore.settings.boardTypeSettings['3d'].pieceScale,
  set: (value) => {
    if (typeof value !== 'number') return;
    settingsStore.settings.boardTypeSettings['3d'].pieceScale = value;
  },
});
</script>
<template>
  <div class="flex flex-col gap-4 w-full">
    <Divider>General Settings</Divider>
    <IftaLabel>
      <Select
        v-model="themeModel"
        :options="themeOptions"
        option-label="label"
        option-value="value"
        fluid
      />
      <label>Theme</label>
    </IftaLabel>
    <IftaLabel>
      <Select
        v-model="darkModeModel"
        :options="darkModeOptions"
        option-label="label"
        option-value="value"
        fluid
      />
      <label>Color Scheme</label>
    </IftaLabel>
    <div class="grid gap-4 items-center" :style="{ gridTemplateColumns: 'auto 1fr auto' }">
      <p>Audio Volume</p>
      <Slider v-model="audioVolumeModel" :min="0" :max="1" :step="0.01" />
      <p class="min-w-12 text-right">
        {{ typeof audioVolumeModel === 'number' ? (audioVolumeModel * 100).toFixed(0) : '' }}%
      </p>
    </div>
    <IftaLabel>
      <Select
        v-model="boardModel"
        :options="boardOptions"
        option-label="label"
        option-value="value"
        fluid
      />
      <label>Board Type</label>
    </IftaLabel>
    <Divider>{{
      { '2d': '2D Settings', '3d': '3D Settings', ninja: 'Ninja Settings' }[boardModel]
    }}</Divider>
    <template v-if="boardModel === '2d'">
      <IftaLabel>
        <Select
          v-model="board2dThemeModel"
          :options="board2dThemeOptions"
          option-label="label"
          option-value="value"
          fluid
        ></Select>
        <label>Theme</label>
      </IftaLabel>
      <div class="grid gap-2 items-center" :style="{ gridTemplateColumns: 'auto 1fr' }">
        <ToggleButton
          v-model="board2dAxisLabels"
          :pt="{
            root: {
              draggable: false,
            },
          }"
          on-label="Axis Labels"
          off-label="Axis Labels"
        />
        <Slider v-model="board2dAxisLabelSize" :disabled="!board2dAxisLabels" />
      </div>
    </template>
    <template v-else-if="boardModel === 'ninja'">
      <IftaLabel>
        <Select
          v-model="boardNinjaThemeModel"
          :options="boardNinjaThemeOptions"
          option-label="label"
          option-value="value"
          fluid
        ></Select>
        <label>Theme</label>
      </IftaLabel>
      <IftaLabel>
        <Select
          v-model="boardNinjaAxisLabels"
          :options="boardNinjaAxisLabelOptions"
          option-label="label"
          option-value="value"
          fluid
        ></Select>
        <label>Axis Labels</label>
      </IftaLabel>
      <ToggleButton
        v-model="boardNinjaAnimateBoard"
        :pt="{
          root: {
            draggable: false,
          },
        }"
        on-label="Animations On"
        off-label="Animations Off"
      />
    </template>
    <template v-else-if="boardModel === '3d'">
      <IftaLabel>
        <Select
          v-model="board3dPiecePresetModel"
          :options="board3dPiecePresetOptions"
          option-label="label"
          option-value="value"
          fluid
        ></Select>
        <label>Piece Preset</label>
      </IftaLabel>
      <IftaLabel>
        <Select
          v-model="board3dTilesPresetModel"
          :options="board3dTilesPresetOptions"
          option-label="label"
          option-value="value"
          fluid
        ></Select>
        <label>Tiles Preset</label>
      </IftaLabel>
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
