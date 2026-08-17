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
<style lang="scss" scoped>
$states: (
  'normal': '.p-slider',
  'hovered': '.p-slider:hover',
  'pressed': '.p-slider:active',
  'disabled': '.p-slider.p-slider-disabled',
);
@each $state, $state-selector in $states {
  #{$state-selector} {
    height: max(
      var(--p-slider-#{$state}-track-height, var(--p-slider-normal-track-height)),
      var(--p-slider-#{$state}-handle-height, var(--p-slider-normal-handle-height))
    );
    padding-left: var(--p-slider-#{$state}-track-padding, var(--p-slider-normal-track-padding));
    padding-right: var(--p-slider-#{$state}-track-padding, var(--p-slider-normal-track-padding));
    opacity: var(--p-slider-#{$state}-opacity, var(--p-slider-normal-opacity));
    .p-slider-track-unfilled-inner {
      background-color: var(
        --p-slider-#{$state}-track-unfilled-background,
        var(--p-slider-normal-track-unfilled-background)
      );
      left: var(--p-slider-#{$state}-track-gap, var(--p-slider-normal-track-gap));
      border-radius: var(
        --p-slider-#{$state}-track-border-radius-inner,
        var(--p-slider-normal-track-border-radius-inner)
      );
    }
    .p-slider-track-filled-inner {
      background-color: var(
        --p-slider-#{$state}-track-filled-background,
        var(--p-slider-normal-track-filled-background)
      );
      right: var(--p-slider-#{$state}-track-gap, var(--p-slider-normal-track-gap));
      border-radius: var(
        --p-slider-#{$state}-track-border-radius-inner,
        var(--p-slider-normal-track-border-radius-inner)
      );
    }
    .p-slider-handle {
      background-color: var(
        --p-slider-#{$state}-handle-background,
        var(--p-slider-normal-handle-background)
      );
      width: var(--p-slider-#{$state}-handle-width, var(--p-slider-normal-handle-width));
      height: var(--p-slider-#{$state}-handle-height, var(--p-slider-normal-handle-height));
      border-radius: var(
        --p-slider-#{$state}-handle-border-radius,
        var(--p-slider-normal-handle-border-radius)
      );
    }
    .p-slider-track {
      border-radius: var(
        --p-slider-#{$state}-track-border-radius,
        var(--p-slider-normal-track-border-radius)
      );
      height: var(--p-slider-#{$state}-track-height, var(--p-slider-normal-track-height));
    }
  }
}

.p-slider {
  touch-action: none;
  outline: none;
  position: relative;
  user-select: none;
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
  right: 0;
  top: 0;
  bottom: 0;
  transition:
    background-color 0.15s ease,
    left 0.15s ease,
    border-radius 0.15s ease;
}
.p-slider-track-filled-inner {
  position: absolute;
  top: 0;
  left: 0;
  bottom: 0;
  transition:
    background-color 0.15s ease,
    right 0.15s ease,
    border-radius 0.15s ease;
}
.p-slider-handle {
  position: absolute;
  top: 50%;
  left: 0;
  transform: translate(-50%, -50%);
  transition:
    background-color 0.15s ease,
    width 0.15s ease,
    height 0.15s ease,
    border-radius 0.15s ease;
}
.p-slider:has(> .p-slider-input:focus-visible) .p-slider-handle {
  outline: var(--p-slider-focus-outline);
  outline-offset: var(--p-slider-focus-outline-offset);
}
</style>
