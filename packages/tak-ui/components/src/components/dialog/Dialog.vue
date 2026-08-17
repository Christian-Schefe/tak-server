<script setup lang="ts">
import { useTemplateRef } from 'vue';
import { Button } from '../button';
import { Icon } from '../icon';
import { useOverlayZIndex } from '../../overlay';

const props = withDefaults(
  defineProps<{
    dismissable?: boolean | undefined;
    header?: string | undefined;
  }>(),
  {
    dismissable: true,
    header: undefined,
  },
);
const visible = defineModel<boolean>('visible', { default: false });
function onClickMask() {
  if (props.dismissable) {
    visible.value = false;
  }
}
const floating = useTemplateRef<HTMLElement | null>('floating');
const zIndex = useOverlayZIndex(floating, visible, 1);
</script>
<template>
  <Teleport to="body">
    <Transition>
      <div
        v-if="visible"
        ref="floating"
        class="p-dialog-mask"
        :style="{ zIndex }"
        @click="onClickMask"
      >
        <div class="p-dialog" @click.stop>
          <div class="p-dialog-header">
            <slot name="header">
              <p class="p-dialog-header-title">{{ header }}</p>
            </slot>
            <Button icon-only variant="text" @click="visible = false"><Icon name="close" /></Button>
          </div>
          <div class="p-dialog-content">
            <slot />
          </div>
          <div v-if="$slots['footer']" class="p-dialog-footer">
            <slot name="footer" />
          </div>
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
.v-enter-active .p-dialog,
.v-leave-active .p-dialog {
  transition: transform 0.15s ease;
}

.v-enter-from,
.v-leave-to {
  opacity: 0;
}

.v-enter-from .p-dialog,
.v-leave-to .p-dialog {
  transform: scale(0.9);
}
.p-dialog-mask {
  position: fixed;
  top: 0;
  left: 0;
  bottom: 0;
  right: 0;
  background-color: var(--p-dialog-mask-background);
  display: flex;
  justify-content: center;
  align-items: center;
}
.p-dialog {
  background-color: var(--p-dialog-background);
  border-radius: var(--p-dialog-border-radius);
  padding-top: var(--p-dialog-padding);
  padding-bottom: var(--p-dialog-padding);
  gap: var(--p-dialog-padding);
  max-width: 40rem;
  width: 100%;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
}
.p-dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--p-dialog-padding);
  padding-left: var(--p-dialog-padding);
  padding-right: var(--p-dialog-padding);
}
.p-dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--p-dialog-padding);
  padding-left: var(--p-dialog-padding);
  padding-right: var(--p-dialog-padding);
}
.p-dialog-header-title {
  font-weight: 600;
  width: 0;
  flex-grow: 1;
  text-overflow: ellipsis;
  overflow: hidden;
  font-size: var(--p-text-large-size);
  line-height: var(--p-text-large-line-height);
}
.p-dialog-content {
  overflow-y: auto;
  min-height: 0;
  padding-left: var(--p-dialog-padding);
  padding-right: var(--p-dialog-padding);
}
</style>
