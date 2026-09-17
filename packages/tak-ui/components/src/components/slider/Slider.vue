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
          <div class="p-slider-track-unfilled-inner"></div>
        </div>
        <div class="p-slider-track-filled" :style="{ right: `${100 - offset * 100}%` }">
          <div class="p-slider-track-filled-inner"></div>
        </div>
      </div>
      <div class="p-slider-handle" :style="{ left: `${offset * 100}%` }" />
    </div>
  </div>
</template>
