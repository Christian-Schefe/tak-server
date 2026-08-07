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
  <div class="p-toggle" :class="{ 'p-toggle-active': value, 'p-toggle-disabled': disabled }">
    <input
      :id="inputId"
      v-model="value"
      class="p-toggle-input"
      type="checkbox"
      :disabled="disabled"
    />
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
  'on': '.p-toggle-active',
);
@each $variant, $variant-selector in $variants {
  @each $state, $state-selector in $states {
    #{$variant-selector}#{$state-selector} {
      width: var(--p-toggle-#{$state}-track-width, var(--p-toggle-#{$variant}-normal-track-width));
      height: var(
        --p-toggle-#{$variant}-#{$state}-track-height,
        var(--p-toggle-#{$variant}-normal-track-height)
      );
      background-color: var(
        --p-toggle-#{$variant}-#{$state}-track-background,
        var(--p-toggle-#{$variant}-normal-track-background)
      );
      border-radius: var(
        --p-toggle-#{$variant}-#{$state}-track-border-radius,
        var(--p-toggle-#{$variant}-normal-track-border-radius)
      );
      border: var(
        --p-toggle-#{$variant}-#{$state}-track-border,
        var(--p-toggle-#{$variant}-normal-track-border)
      );
      outline: var(
        --p-toggle-#{$variant}-#{$state}-track-outline,
        var(--p-toggle-#{$variant}-normal-track-outline)
      );
      outline-offset: var(
        --p-toggle-#{$variant}-#{$state}-track-outline-offset,
        var(--p-toggle-#{$variant}-normal-track-outline-offset)
      );
      .p-toggle-handle {
        width: var(
          --p-toggle-#{$variant}-#{$state}-handle-width,
          var(--p-toggle-#{$variant}-normal-handle-width)
        );
        height: var(
          --p-toggle-#{$variant}-#{$state}-handle-height,
          var(--p-toggle-#{$variant}-normal-handle-height)
        );
        background-color: var(
          --p-toggle-#{$variant}-#{$state}-handle-background,
          var(--p-toggle-#{$variant}-normal-handle-background)
        );
        border-radius: var(
          --p-toggle-#{$variant}-#{$state}-handle-border-radius,
          var(--p-toggle-#{$variant}-normal-handle-border-radius)
        );
        border: var(
          --p-toggle-#{$variant}-#{$state}-handle-border,
          var(--p-toggle-#{$variant}-normal-handle-border)
        );
      }
    }
  }
}

.p-toggle {
  transition:
    background-color 0.2s ease,
    border 0.2s ease,
    border-radius 0.2s ease;
  position: relative;
}

.p-toggle:has(> .p-toggle-input:focus-visible) {
  outline: var(--p-toggle-focus-outline);
  outline-offset: var(--p-toggle-focus-outline-offset);
}

.p-toggle-handle {
  top: 50%;
  left: calc(var(--p-toggle-off-normal-track-height) / 2);
  position: absolute;
  transition:
    left 0.2s cubic-bezier(0.175, 0.885, 0.32, 1.275),
    background-color 0.2s ease,
    width 0.2s ease,
    height 0.2s ease,
    border-radius 0.2s ease,
    border 0.2s ease;
  transform: translate(-50%, -50%);
}
.p-toggle.p-toggle-active .p-toggle-handle {
  left: calc(100% - var(--p-toggle-on-normal-track-height) / 2);
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
