<script setup lang="ts">
import {
  autoUpdate,
  flip,
  limitShift,
  offset,
  shift,
  useFloating,
  type Placement,
} from '@floating-ui/vue';
import { computed, onMounted, onUnmounted, useTemplateRef } from 'vue';
import { useOverlayZIndex } from '../../overlay';

const dropdownVisible = defineModel<boolean>({ default: false });
const props = withDefaults(
  defineProps<{
    reference?: HTMLElement | null;
    placement?: Placement;
  }>(),
  {
    reference: null,
    placement: 'bottom-start',
  },
);

function onPointerDownOutside(event: PointerEvent) {
  if (
    dropdownVisible.value &&
    props.reference?.contains(event.target as Node) !== true &&
    floating.value?.contains(event.target as Node) !== true
  ) {
    dropdownVisible.value = false;
  }
}

const referenceRef = computed(() => props.reference);

const floating = useTemplateRef<HTMLElement | null>('floating');
const { floatingStyles } = useFloating(referenceRef, floating, {
  placement: () => props.placement,
  middleware: [offset(10), flip({ padding: 10 }), shift({ limiter: limitShift(), padding: 10 })],
  whileElementsMounted: autoUpdate,
});

const zIndex = useOverlayZIndex(floating, dropdownVisible, 1);

onMounted(() => {
  document.addEventListener('pointerdown', onPointerDownOutside);
});
onUnmounted(() => {
  document.removeEventListener('pointerdown', onPointerDownOutside);
});
</script>
<template>
  <Teleport to="body">
    <Transition>
      <div v-if="dropdownVisible" ref="floating" :style="{ ...floatingStyles, zIndex }">
        <div class="p-dropdown">
          <slot />
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
<style lang="css" scoped>
.v-enter-active,
.v-leave-active {
  transition: opacity 0.15s ease;
}

.v-enter-active .p-dropdown,
.v-leave-active .p-dropdown {
  transition: transform 0.15s ease;
}

.v-enter-from,
.v-leave-to {
  opacity: 0;
}

.v-enter-from .p-dropdown,
.v-leave-to .p-dropdown {
  transform: var(--p-dropdown-transform-enter-from);
}
</style>
