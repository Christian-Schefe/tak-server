<script setup lang="ts">
import { computed } from 'vue';

const props = withDefaults(
  defineProps<{
    min?: number;
    max?: number;
    step?: number | undefined;
    disabled?: boolean;
  }>(),
  {
    min: 0,
    max: 100,
    step: undefined,
    disabled: false,
  },
);
const value = defineModel<number>({ default: 0 });

const offset = computed(() => {
  const clampedValue = Math.min(Math.max(value.value, props.min), props.max);
  const diff = props.max - props.min;
  return diff !== 0 ? (clampedValue - props.min) / diff : 0;
});
</script>
<template>
  <div class="p-slider" :class="{ 'p-slider-disabled': disabled }">
    <input
      v-model="value"
      class="p-slider-input"
      type="range"
      :min="min"
      :max="max"
      :step="step ?? 'any'"
      :disabled="disabled"
    />
    <div class="p-slider-inner">
      <div class="p-slider-track">
        <div class="p-slider-track-unfilled" />
        <div class="p-slider-track-inner">
          <div class="p-slider-track-filled" :style="{ right: `${100 - offset * 100}%` }" />
        </div>
        <span class="p-slider-handle" :style="{ left: `${offset * 100}%` }" />
      </div>
    </div>
  </div>
</template>
<style lang="css" scoped>
.p-slider {
  height: max(var(--p-slider-height), var(--p-slider-handle-size));
  touch-action: none;
  padding-left: calc(max(var(--p-slider-height), var(--p-slider-handle-size)) * 0.5);
  padding-right: calc(max(var(--p-slider-height), var(--p-slider-handle-size)) * 0.5);
  outline: none;
  cursor: pointer;
  position: relative;
}
.p-slider.p-slider-disabled {
  cursor: unset;
}
.p-slider:hover .p-slider-track-unfilled {
  background-color: var(--p-slider-hover-background);
}
.p-slider:hover .p-slider-track-filled {
  background-color: var(--p-slider-hover-filled-background);
}
.p-slider.p-slider-disabled .p-slider-track-unfilled {
  background-color: var(--p-slider-disabled-background);
}
.p-slider.p-slider-disabled .p-slider-track-filled {
  background-color: var(--p-slider-disabled-filled-background);
}
.p-slider.p-slider-disabled .p-slider-handle {
  background-color: var(--p-slider-handle-disabled-background);
}
.p-slider-input {
  position: absolute;
  opacity: 0;
  top: 0;
  left: 0;
  bottom: 0;
  right: 0;
  z-index: 1;
}
.p-slider-inner {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
}
.p-slider-track {
  position: relative;
  width: 100%;
  height: var(--p-slider-height);
}
.p-slider-track-unfilled {
  position: absolute;
  left: calc(var(--p-slider-height) * -0.5);
  right: calc(var(--p-slider-height) * -0.5);
  height: 100%;
  background-color: var(--p-slider-background);
  border-radius: var(--p-slider-height);
  transition: background-color 0.2s ease;
}
.p-slider-track-inner {
  position: absolute;
  top: 0;
  left: calc(var(--p-slider-height) * 0.5);
  right: calc(var(--p-slider-height) * -0.5);
  bottom: 0;
}
.p-slider-track-filled {
  position: absolute;
  top: 0;
  left: calc(var(--p-slider-height) * -1);
  right: 0;
  bottom: 0;
  background-color: var(--p-slider-filled-background);
  border-radius: var(--p-slider-height);
  transition: background-color 0.2s ease;
}
.p-slider-handle {
  position: absolute;
  top: 50%;
  left: 0;
  transform: translate(-50%, -50%);
  width: var(--p-slider-handle-size);
  height: var(--p-slider-handle-size);
  background-color: var(--p-slider-handle-background);
  border-radius: 50%;
  border: var(--p-slider-handle-border, none);
}
.p-slider:has(> .p-slider-input:focus-visible) .p-slider-handle {
  outline: var(--p-slider-handle-focus-outline);
  outline-offset: var(--p-slider-handle-focus-outline-offset);
}
</style>
