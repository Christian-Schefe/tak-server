<script setup lang="ts">
import { computed, useTemplateRef } from 'vue';
import { useOverlayZIndex } from '../../overlay';

const props = withDefaults(
  defineProps<{
    size?: string | undefined;
    direction?: 'left' | 'right' | 'top' | 'bottom';
    overlay?: boolean | undefined;
  }>(),
  {
    direction: 'left',
    size: undefined,
    overlay: false,
  },
);
const visible = defineModel<boolean>('visible', { default: false });
function onClickMask() {
  if (props.overlay) {
    visible.value = false;
  }
}
const actualSize = computed(
  () =>
    props.size ?? (props.direction === 'left' || props.direction === 'right' ? '256px' : '56px'),
);
const floating = useTemplateRef<HTMLElement | null>('floating');
const zIndex = useOverlayZIndex(floating, visible, 0);
</script>
<template>
  <Transition>
    <div
      v-if="visible"
      ref="floating"
      :class="{
        'p-sidebar': true,
        [`p-sidebar-${direction}`]: true,
        [`p-sidebar-overlay-${overlay}`]: true,
      }"
      :style="{ '--p-sidebar-size': actualSize, zIndex }"
      @click="onClickMask"
    >
      <div class="p-sidebar-inner" @click.stop>
        <slot />
      </div>
    </div>
  </Transition>
</template>
