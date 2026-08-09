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
      :style="{ '--p-sidebar-_size': actualSize, zIndex }"
      @click="onClickMask"
    >
      <div class="p-sidebar-inner" @click.stop>
        <slot />
      </div>
    </div>
  </Transition>
</template>
<style lang="css" scoped>
.p-sidebar.v-enter-active,
.p-sidebar.v-leave-active {
  transition:
    opacity 0.2s ease,
    width 0.2s ease,
    height 0.2s ease;
}

.p-sidebar.v-enter-active .p-sidebar-inner,
.p-sidebar.v-leave-active .p-sidebar-inner {
  transition: transform 0.2s ease;
}

.p-sidebar.v-enter-from.p-sidebar-overlay-true,
.p-sidebar.v-leave-to.p-sidebar-overlay-true {
  opacity: 0;
}

.p-sidebar.v-enter-from.p-sidebar-left.p-sidebar-overlay-false,
.p-sidebar.v-leave-to.p-sidebar-left.p-sidebar-overlay-false,
.p-sidebar.v-enter-from.p-sidebar-right.p-sidebar-overlay-false,
.p-sidebar.v-leave-to.p-sidebar-right.p-sidebar-overlay-false {
  width: 0;
}
.p-sidebar.v-enter-from.p-sidebar-top.p-sidebar-overlay-false,
.p-sidebar.v-leave-to.p-sidebar-top.p-sidebar-overlay-false,
.p-sidebar.v-enter-from.p-sidebar-bottom.p-sidebar-overlay-false,
.p-sidebar.v-leave-to.p-sidebar-bottom.p-sidebar-overlay-false {
  height: 0;
}

.p-sidebar.v-enter-from.p-sidebar-left .p-sidebar-inner,
.p-sidebar.v-leave-to.p-sidebar-left .p-sidebar-inner {
  transform: translateX(-100%);
}
.p-sidebar.v-enter-from.p-sidebar-right .p-sidebar-inner,
.p-sidebar.v-leave-to.p-sidebar-right .p-sidebar-inner {
  transform: translateX(100%);
}
.p-sidebar.v-enter-from.p-sidebar-top .p-sidebar-inner,
.p-sidebar.v-leave-to.p-sidebar-top .p-sidebar-inner {
  transform: translateY(-100%);
}
.p-sidebar.v-enter-from.p-sidebar-bottom .p-sidebar-inner,
.p-sidebar.v-leave-to.p-sidebar-bottom .p-sidebar-inner {
  transform: translateY(100%);
}

.p-sidebar {
  position: relative;
}
.p-sidebar.p-sidebar-overlay-true {
  background-color: var(--p-sidebar-mask-background);
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
}
.p-sidebar.p-sidebar-left.p-sidebar-overlay-false,
.p-sidebar.p-sidebar-right.p-sidebar-overlay-false {
  width: var(--p-sidebar-_size);
}
.p-sidebar.p-sidebar-top.p-sidebar-overlay-false,
.p-sidebar.p-sidebar-bottom.p-sidebar-overlay-false {
  height: var(--p-sidebar-_size);
}

.p-sidebar-inner {
  position: absolute;
  display: flex;
  background-color: var(--p-sidebar-background);
  color: var(--p-sidebar-text);
  padding: var(--p-sidebar-padding);
  gap: var(--p-sidebar-padding);
}

.p-sidebar.p-sidebar-left .p-sidebar-inner {
  border-right: var(--p-sidebar-border);
  flex-direction: column;
  top: 0;
  bottom: 0;
  left: 0;
  width: var(--p-sidebar-_size);
}
.p-sidebar.p-sidebar-right .p-sidebar-inner {
  border-left: var(--p-sidebar-border);
  flex-direction: column;
  top: 0;
  bottom: 0;
  right: 0;
  width: var(--p-sidebar-_size);
}
.p-sidebar.p-sidebar-top .p-sidebar-inner {
  border-bottom: var(--p-sidebar-border);
  flex-direction: row;
  align-items: center;
  top: 0;
  left: 0;
  right: 0;
  height: var(--p-sidebar-_size);
}
.p-sidebar.p-sidebar-bottom .p-sidebar-inner {
  border-top: var(--p-sidebar-border);
  flex-direction: row;
  align-items: center;
  bottom: 0;
  left: 0;
  right: 0;
  height: var(--p-sidebar-_size);
}
</style>
