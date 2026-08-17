<script setup lang="ts">
const value = defineModel<boolean>({ default: false });
withDefaults(
  defineProps<{
    disabled?: boolean | undefined;
    inputId?: string | undefined;
  }>(),
  {
    disabled: false,
    inputId: undefined,
  },
);
</script>
<template>
  <div class="p-toggle" :class="{ 'p-toggle-on': value, 'p-toggle-disabled': disabled }">
    <input
      :id="inputId"
      v-model="value"
      class="p-toggle-input"
      type="checkbox"
      :disabled="disabled"
    />
    <div class="p-toggle-track" />
    <div class="p-toggle-handle" />
  </div>
</template>
<style lang="scss" scoped>
$states: (
  'normal': '.p-toggle',
  'hovered': '.p-toggle:hover',
  'pressed': '.p-toggle:active',
  'disabled': '.p-toggle.p-toggle-disabled',
);
$variants: (
  'off': '',
  'on': '.p-toggle-on',
);
@each $state, $state-selector in $states {
  @each $variant, $variant-selector in $variants {
    #{$state-selector}#{$variant-selector} {
      .p-toggle-track {
        background-color: var(
          --p-toggle-#{$state}-#{$variant}-track-background,
          var(--p-toggle-normal-#{$variant}-track-background)
        );
        border-radius: var(
          --p-toggle-#{$state}-#{$variant}-track-border-radius,
          var(--p-toggle-normal-#{$variant}-track-border-radius)
        );
        border: var(
          --p-toggle-#{$state}-#{$variant}-track-border,
          var(--p-toggle-normal-#{$variant}-track-border)
        );
        outline: var(
          --p-toggle-#{$state}-#{$variant}-track-outline,
          var(--p-toggle-normal-#{$variant}-track-outline)
        );
        outline-offset: var(
          --p-toggle-#{$state}-#{$variant}-track-outline-offset,
          var(--p-toggle-normal-#{$variant}-track-outline-offset)
        );
      }
      .p-toggle-handle {
        width: var(
          --p-toggle-#{$state}-#{$variant}-handle-width,
          var(--p-toggle-normal-#{$variant}-handle-width)
        );
        height: var(
          --p-toggle-#{$state}-#{$variant}-handle-height,
          var(--p-toggle-normal-#{$variant}-handle-height)
        );
        background-color: var(
          --p-toggle-#{$state}-#{$variant}-handle-background,
          var(--p-toggle-normal-#{$variant}-handle-background)
        );
        border-radius: var(
          --p-toggle-#{$state}-#{$variant}-handle-border-radius,
          var(--p-toggle-normal-#{$variant}-handle-border-radius)
        );
        border: var(
          --p-toggle-#{$state}-#{$variant}-handle-border,
          var(--p-toggle-normal-#{$variant}-handle-border)
        );
      }
    }
  }
  #{$state-selector} {
    width: var(--p-toggle-#{$state}-track-width, var(--p-toggle-normal-track-width));
    height: var(--p-toggle-#{$state}-track-height, var(--p-toggle-normal-track-height));
    opacity: var(--p-toggle-#{$state}-opacity, var(--p-toggle-normal-opacity));
  }
  #{$state-selector} .p-toggle-handle {
    left: calc(var(--p-toggle-#{$state}-track-height, var(--p-toggle-normal-track-height)) / 2);
  }
  #{$state-selector}.p-toggle-on .p-toggle-handle {
    left: calc(
      100% - var(--p-toggle-#{$state}-track-height, var(--p-toggle-normal-track-height)) / 2
    );
  }
}

.p-toggle {
  position: relative;
}

.p-toggle-track {
  width: 100%;
  height: 100%;
  transition:
    background-color 0.15s ease,
    border 0.15s ease,
    border-radius 0.15s ease;
}

.p-toggle:has(> .p-toggle-input:focus-visible) {
  outline: var(--p-toggle-focus-outline);
  outline-offset: var(--p-toggle-focus-outline-offset);
}

.p-toggle-handle {
  top: 50%;
  position: absolute;
  transition:
    left 0.15s cubic-bezier(0.175, 0.885, 0.32, 1.275),
    background-color 0.15s ease,
    width 0.15s ease,
    height 0.15s ease,
    border-radius 0.15s ease,
    border 0.15s ease;
  transform: translate(-50%, -50%);
}

.p-toggle-input {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  opacity: 0;
  z-index: 1;
  cursor: pointer;
}
.p-toggle.p-toggle-disabled .p-toggle-input {
  cursor: unset;
}
</style>
