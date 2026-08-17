<script setup lang="ts" generic="T extends Component | keyof IntrinsicElementAttributes">
import { computed, type Component, type IntrinsicElementAttributes, type StyleValue } from 'vue';
import type { ComponentProps } from 'vue-component-type-helpers';

type PropsOf<T> = T extends Component
  ? ComponentProps<T>
  : T extends keyof IntrinsicElementAttributes
    ? IntrinsicElementAttributes[T]
    : never;

const props = withDefaults(
  defineProps<{
    severity?: 'primary' | 'secondary' | undefined;
    variant?: 'filled' | 'text' | 'outlined' | undefined;
    align?: 'start' | 'center' | 'end' | undefined;
    disabled?: boolean | undefined;
    iconOnly?: boolean | undefined;
    label?: string | undefined;
    type?: 'button' | 'submit' | 'reset';
    as?: undefined | { component: T; props: PropsOf<T> };
  }>(),
  {
    severity: 'primary',
    variant: 'filled',
    align: 'start',
    disabled: false,
    iconOnly: false,
    label: undefined,
    type: 'button',
    as: undefined,
  },
);

const emit = defineEmits<{
  click: [PointerEvent];
}>();

function handlePointerDown(e: PointerEvent) {
  if (props.disabled) {
    e.preventDefault();
    return;
  }
  const button = e.currentTarget as HTMLElement | null;
  if (button) {
    const rect = button.getBoundingClientRect();

    const ripple = document.createElement('span');
    const size = Math.max(rect.width, rect.height);

    ripple.style.width = `${size}px`;
    ripple.style.height = `${size}px`;
    ripple.style.left = `${e.clientX - rect.left - size / 2}px`;
    ripple.style.top = `${e.clientY - rect.top - size / 2}px`;
    ripple.style.setProperty(
      '--p-button-_active',
      `var(--p-button-${props.variant}-${props.severity}-active)`,
    );
    ripple.className = 'p-button-ripple';

    button.appendChild(ripple);
    window.addEventListener(
      'pointerup',
      () => {
        ripple.classList.add('p-button-ripple-fadeout');
        ripple.addEventListener('animationend', () => {
          ripple.remove();
        });
      },
      { once: true },
    );
  }
}

function handleClick(e: PointerEvent) {
  if (props.disabled) {
    e.preventDefault();
    return;
  }
  emit('click', e);
}

const buttonContentStyle = computed<StyleValue>(() => {
  return {
    justifyContent:
      props.align === 'start' ? 'flex-start' : props.align === 'end' ? 'flex-end' : 'center',
    width: props.iconOnly ? `var(--p-button-size)` : 'auto',
    height: props.iconOnly ? `var(--p-button-size)` : 'auto',
  };
});
</script>

<template>
  <component
    :is="props.as?.component ?? 'button'"
    class="p-button"
    :class="[
      `p-button-${props.variant}`,
      `p-button-${props.severity}`,
      { 'p-button-disabled': props.disabled, 'p-button-icon-only': props.iconOnly },
    ]"
    :disabled="disabled"
    :draggable="false"
    :type="type"
    v-bind="props.as?.props"
    @click="handleClick"
    @pointerdown="handlePointerDown"
  >
    <div class="p-button-content" :style="buttonContentStyle">
      <slot name="icon" />
      <slot>
        <p class="p-button-label">{{ label }}</p>
      </slot>
    </div>
  </component>
</template>
<style lang="scss">
$variants: (
  'filled': '.p-button-filled',
  'text': '.p-button-text',
  'outlined': '.p-button-outlined',
);
$severities: (
  'primary': '.p-button-primary',
  'secondary': '.p-button-secondary',
);

@each $severity, $severity-selector in $severities {
  @each $variant, $variant-selector in $variants {
    #{$variant-selector}#{$severity-selector} .p-button-ripple {
      background-color: var(
        --p-button-pressed-#{$variant}-#{$severity}-background,
        var(--p-button-normal-#{$variant}-#{$severity}-background)
      );
    }
  }
}
.p-button-ripple {
  position: absolute;
  border-radius: 50%;
  transform: scale(2.5);
  pointer-events: none;
  animation: ripple-animation 0.2s linear;
}
@keyframes ripple-animation {
  from {
    transform: scale(0);
  }
  to {
    transform: scale(2.5);
  }
}
@keyframes fadeout-animation {
  to {
    opacity: 0;
  }
}
.p-button-ripple-fadeout {
  animation: fadeout-animation 0.15s linear;
}
</style>
<style lang="scss" scoped>
$states: (
  'normal': '.p-button',
  'hovered': '.p-button:hover',
  'pressed': '.p-button:active',
  'disabled': '.p-button.p-button-disabled',
);
$variants: (
  'filled': '.p-button-filled',
  'text': '.p-button-text',
  'outlined': '.p-button-outlined',
);
$severities: (
  'primary': '.p-button-primary',
  'secondary': '.p-button-secondary',
);
@each $state, $state-selector in $states {
  @each $severity, $severity-selector in $severities {
    @each $variant, $variant-selector in $variants {
      #{$state-selector}#{$variant-selector}#{$severity-selector} {
        background-color: var(
          --p-button-#{$state}-#{$variant}-#{$severity}-background,
          var(--p-button-normal-#{$variant}-#{$severity}-background),
        );
        color: var(
          --p-button-#{$state}-#{$variant}-#{$severity}-text,
          var(--p-button-normal-#{$variant}-#{$severity}-text),
        );
        border: var(
          --p-button-#{$state}-#{$variant}-#{$severity}-border,
          var(--p-button-normal-#{$variant}-#{$severity}-border, none),
        );
      }
    }
  }
  #{$state-selector} {
    border-radius: var(--p-button-#{$state}-border-radius, var(--p-button-normal-border-radius));
    opacity: var(--p-button-#{$state}-opacity, var(--p-button-normal-opacity));
    padding: var(--p-button-#{$state}-padding, var(--p-button-normal-padding))
      calc(var(--p-button-#{$state}-padding, var(--p-button-normal-padding)) + 0.25rem);
  }
  #{$state-selector}.p-button-icon-only {
    padding: var(--p-button-#{$state}-padding, var(--p-button-normal-padding));
  }
  #{$state-selector} .p-button-content {
    gap: var(--p-button-#{$state}-gap, var(--p-button-normal-gap, 0.25rem));
  }
}
.p-button {
  margin: 0;
  transition:
    background-color 0.15s ease-in-out,
    color 0.15s ease-in-out,
    border 0.15s ease-in-out;
  position: relative;
  overflow: hidden;
  cursor: pointer;
}
.p-button:disabled {
  cursor: unset;
}

.p-button-group .p-button:not(:last-child) {
  border-top-right-radius: 0;
  border-bottom-right-radius: 0;
}
.p-button-group .p-button:not(:first-child) {
  border-top-left-radius: 0;
  border-bottom-left-radius: 0;
}

.p-button .p-button-content {
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1;
  position: relative;
}

.p-button:focus-visible {
  outline: var(--p-button-focus-outline);
  outline-offset: var(--p-button-focus-outline-offset);
}

.p-button-label {
  font-weight: 600;
}
</style>
