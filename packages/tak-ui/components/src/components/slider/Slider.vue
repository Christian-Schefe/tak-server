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

function handleInput(event: InputEvent) {
  const target = event.target as HTMLInputElement | null;
  if (target) {
    const numberValue = Number(target.value);
    if (!isNaN(numberValue)) {
      value.value = Math.min(Math.max(numberValue, props.min), props.max);
    }
  }
}
</script>
<template>
  <div class="p-slider" :class="{ 'p-slider-disabled': disabled }">
    <input
      :value="value"
      class="p-slider-input"
      type="range"
      :min="min"
      :max="max"
      :step="step ?? 'any'"
      :disabled="disabled"
      @input="handleInput"
    />
    <div class="p-slider-inner">
      <div class="p-slider-track">
        <div class="p-slider-track-unfilled" :style="{ left: `${offset * 100}%` }">
          <div class="p-slider-track-unfilled-inner" />
        </div>
        <div class="p-slider-track-filled" :style="{ right: `${100 - offset * 100}%` }">
          <div class="p-slider-track-filled-inner" />
        </div>
      </div>
      <div class="p-slider-handle" :style="{ left: `${offset * 100}%` }" />
    </div>
  </div>
</template>
<style lang="css" scoped>
.p-slider {
  height: max(var(--p-slider-height), var(--p-slider-handle-height));
  touch-action: none;
  padding-left: var(--p-slider-padding);
  padding-right: var(--p-slider-padding);
  outline: none;
  position: relative;
  user-select: none;
}
.p-slider:hover .p-slider-track-unfilled-inner {
  background-color: var(--p-slider-hover-background);
}
.p-slider:hover .p-slider-track-filled-inner {
  background-color: var(--p-slider-hover-filled-background);
}
.p-slider:hover .p-slider-handle {
  background-color: var(--p-slider-handle-hover-background);
}
.p-slider.p-slider-disabled .p-slider-track-unfilled-inner {
  background-color: var(--p-slider-disabled-background);
}
.p-slider.p-slider-disabled .p-slider-track-filled-inner {
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
  cursor: pointer;
}
.p-slider.p-slider-disabled .p-slider-input {
  cursor: unset;
}
.p-slider-inner {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  position: relative;
}
.p-slider-track {
  position: relative;
  width: 100%;
  height: var(--p-slider-height);
  border-radius: var(--p-slider-border-radius);
  overflow: hidden;
}
.p-slider-track-unfilled {
  position: absolute;
  right: 0;
  top: 0;
  bottom: 0;
}
.p-slider-track-filled {
  position: absolute;
  top: 0;
  left: 0;
  bottom: 0;
}
.p-slider-track-unfilled-inner {
  position: absolute;
  left: var(--p-slider-gap);
  right: 0;
  top: 0;
  bottom: 0;
  background-color: var(--p-slider-background);
  transition: background-color 0.2s ease;
  border-radius: var(--p-slider-border-radius);
}
.p-slider-track-filled-inner {
  position: absolute;
  top: 0;
  left: 0;
  right: var(--p-slider-gap);
  bottom: 0;
  background-color: var(--p-slider-filled-background);
  transition: background-color 0.2s ease;
  border-radius: var(--p-slider-border-radius);
}
.p-slider-handle {
  position: absolute;
  top: 50%;
  left: 0;
  transform: translate(-50%, -50%);
  width: var(--p-slider-handle-width);
  height: var(--p-slider-handle-height);
  background-color: var(--p-slider-handle-background);
  border-radius: var(--p-slider-handle-border-radius);
  border: var(--p-slider-handle-border, none);
  transition: background-color 0.2s ease;
}
.p-slider:has(> .p-slider-input:focus-visible) .p-slider-handle {
  outline: var(--p-slider-handle-focus-outline);
  outline-offset: var(--p-slider-handle-focus-outline-offset);
}
</style>
