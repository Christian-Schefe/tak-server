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

const buttonStyle = computed<StyleValue>(() => {
  return {
    '--p-button-_background': props.disabled
      ? `var(--p-button-${props.variant}-${props.severity}-disabled-background)`
      : `var(--p-button-${props.variant}-${props.severity}-background)`,
    '--p-button-_hover': `var(--p-button-${props.variant}-${props.severity}-hover)`,
    color: props.disabled
      ? `var(--p-button-${props.variant}-${props.severity}-disabled-text)`
      : `var(--p-button-${props.variant}-${props.severity}-text)`,
    border: `var(--p-button-${props.variant}-${props.severity}-border, none)`,
    padding: props.iconOnly
      ? `var(--p-button-padding)`
      : `var(--p-button-padding) calc(var(--p-button-padding) + 0.25rem)`,
  };
});
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
    :disabled="disabled"
    :draggable="false"
    :style="buttonStyle"
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
<style lang="css">
.p-button-ripple {
  position: absolute;
  border-radius: 50%;
  transform: scale(2.5);
  pointer-events: none;
  animation: ripple-animation 0.2s linear;
  background-color: var(--p-button-_active);
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
  animation: fadeout-animation 0.1s linear;
}
</style>
<style lang="css" scoped>
.p-button {
  margin: 0;
  border-radius: var(--p-button-border-radius);
  background-color: var(--p-button-_background);
  transition:
    background-color 0.1s ease-in-out,
    color 0.1s ease-in-out,
    border 0.1s ease-in-out;
  position: relative;
  overflow: hidden;
  cursor: pointer;
}
.p-button:disabled {
  cursor: unset;
}
.p-button:hover:not(:disabled) {
  background-color: var(--p-button-_hover);
}
.p-button:active:not(:disabled) {
  background-color: var(--p-button-_hover);
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
  gap: var(--p-button-padding);
}

.p-button:focus-visible {
  outline: var(--p-button-focus-outline);
  outline-offset: var(--p-button-focus-outline-offset);
}

.p-button-label {
  font-weight: 600;
}
</style>
