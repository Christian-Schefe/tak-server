<script setup lang="ts" generic="T extends Component | keyof IntrinsicElementAttributes">
import { type Component, type IntrinsicElementAttributes } from 'vue';
import type { ComponentProps } from 'vue-component-type-helpers';

type PropsOf<T> = T extends Component
  ? ComponentProps<T>
  : T extends keyof IntrinsicElementAttributes
    ? IntrinsicElementAttributes[T]
    : never;

const props = withDefaults(
  defineProps<{
    severity?: 'primary' | 'secondary' | 'tertiary' | 'danger';
    variant?: 'filled' | 'text' | 'outlined' | 'tonal';
    size?: 'small' | 'medium';
    disabled?: boolean;
    iconOnly?: boolean;
    label?: string | undefined;
    type?: 'button' | 'submit' | 'reset';
    name?: string | undefined;
    value?: string | undefined;
    as?: undefined | { component: T; props: PropsOf<T> };
  }>(),
  {
    severity: 'primary',
    variant: 'filled',
    size: 'medium',
    disabled: false,
    iconOnly: false,
    label: undefined,
    type: 'button',
    name: undefined,
    value: undefined,
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
    ripple.className = 'p-button-ripple';

    const destroyFunc = () => {
      if (!ripple.isConnected) {
        return;
      }
      ripple.classList.add('p-button-ripple-fadeout');
      ripple.addEventListener('animationend', () => {
        ripple.remove();
      });
    };

    button.appendChild(ripple);
    window.addEventListener('pointerup', destroyFunc, { once: true });
    window.addEventListener('pointercancel', destroyFunc, { once: true });
  }
}

function handleClick(e: PointerEvent) {
  if (props.disabled) {
    e.preventDefault();
    return;
  }
  emit('click', e);
}
</script>

<template>
  <component
    :is="props.as?.component ?? 'button'"
    class="p-button"
    :class="[
      `p-button-${props.variant}`,
      `p-button-${props.severity}`,
      `p-button-${props.size}`,
      { 'p-button-disabled': props.disabled, 'p-button-icon-only': props.iconOnly },
    ]"
    :disabled="disabled"
    :draggable="false"
    :type="type"
    :name="name"
    :value="value"
    v-bind="props.as?.props"
    @click="handleClick"
    @pointerdown="handlePointerDown"
  >
    <div class="p-button-state" />
    <div class="p-button-content">
      <span v-if="$slots['icon']">
        <slot name="icon" />
      </span>
      <p class="p-button-label">
        <slot>{{ label }}</slot>
      </p>
      <span v-if="$slots['icon-append']">
        <slot name="icon-append" />
      </span>
    </div>
  </component>
</template>
