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

const inner = useTemplateRef<HTMLElement | null>('inner');

const visible = defineModel<boolean>('visible', { default: false });
function onClickMask(event: PointerEvent) {
  if (props.dismissable && inner.value?.contains(event.target as Node) !== true) {
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
        @pointerdown="onClickMask"
      >
        <div ref="inner" class="p-dialog">
          <div class="p-dialog-header">
            <slot name="header">
              <p class="p-dialog-header-title">{{ header }}</p>
            </slot>
            <Button icon-only variant="text" @click="visible = false"><Icon name="close" /></Button>
          </div>
          <div class="p-dialog-content">
            <slot />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
