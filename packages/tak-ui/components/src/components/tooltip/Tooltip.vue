<script setup lang="ts">
import {
  arrow,
  autoUpdate,
  flip,
  limitShift,
  offset,
  shift,
  useFloating,
  type Placement,
} from '@floating-ui/vue';
import { useElementHover, useFocusWithin } from '@vueuse/core';
import { computed, useTemplateRef } from 'vue';
import { useOverlayZIndex } from '../../overlay';

const props = withDefaults(
  defineProps<{
    placement?: Placement;
    activation?: 'hover' | 'focus' | boolean;
  }>(),
  {
    placement: 'bottom',
    activation: 'hover',
  },
);

const reference = useTemplateRef<HTMLElement | null>('reference');
const floating = useTemplateRef<HTMLElement | null>('floating');
const arrowEl = useTemplateRef<HTMLElement | null>('floatingArrow');

const {
  floatingStyles,
  middlewareData,
  placement: floatingPlacement,
} = useFloating(reference, floating, {
  placement: () => props.placement,
  middleware: [
    offset(10),
    flip({ padding: 5 }),
    shift({ limiter: limitShift(), padding: 5 }),
    arrow({ element: arrowEl, padding: 5 }),
  ],
  whileElementsMounted: autoUpdate,
});

const arrowStyles = computed(() => {
  const { x, y } = middlewareData.value.arrow ?? {};
  const isVertical =
    floatingPlacement.value.startsWith('top') || floatingPlacement.value.startsWith('bottom');
  return {
    left: x !== undefined ? `${x}px` : floatingPlacement.value.startsWith('left') ? '100%' : '0',
    top: y !== undefined ? `${y}px` : floatingPlacement.value.startsWith('top') ? '100%' : '0',
    transform: `translate(${isVertical ? '0' : '-50%'}, ${isVertical ? '-50%' : '0'}) rotate(45deg)`,
  };
});

const hovered = useElementHover(reference);
const { focused } = useFocusWithin(reference);
const visible = computed(() => {
  switch (props.activation) {
    case 'hover':
      return hovered.value;
    case 'focus':
      return focused.value;
    default:
      return props.activation;
  }
});
const zIndex = useOverlayZIndex(floating, visible, 0);
</script>
<template>
  <div ref="reference">
    <slot></slot>
    <Teleport to="body">
      <Transition>
        <div v-if="visible" ref="floating" :style="{ ...floatingStyles, zIndex }">
          <div class="p-tooltip">
            <slot name="content" />
            <div ref="floatingArrow" class="p-tooltip-arrow" :style="arrowStyles"></div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>
<style lang="css" scoped>
.v-enter-active,
.v-leave-active {
  transition: opacity 0.15s ease;
}
.v-enter-active .p-tooltip,
.v-leave-active .p-tooltip {
  transition: transform 0.15s ease;
}

.v-enter-from,
.v-leave-to {
  opacity: 0;
}

.v-enter-from .p-tooltip,
.v-leave-to .p-tooltip {
  transform: scale(0.9);
}

.p-tooltip-arrow {
  position: absolute;
  width: 0.5rem;
  height: 0.5rem;
  background-color: var(--p-tooltip-background);
}

.p-tooltip {
  background-color: var(--p-tooltip-background);
  color: var(--p-tooltip-text);
  padding: var(--p-tooltip-padding);
  border-radius: var(--p-tooltip-border-radius);
  box-shadow: var(--p-tooltip-box-shadow);
}
</style>
