<script setup lang="ts">
import type { GameSettings } from '@/api/game';
import { isDefaultReserve } from '@/tak-core';
import Button from 'primevue/button';
import Popover from 'primevue/popover';
import { computed, useTemplateRef } from 'vue';
import { Fa6RegChessPawn, Fa6RegChessQueen } from 'vue-icons-plus/fa6';
import { LuInfo, LuPlay } from 'vue-icons-plus/lu';

const props = defineProps<{
  settings: GameSettings;
}>();

const detailsPopover = useTemplateRef('detailsPopover');

function toggle(event: Event) {
  detailsPopover.value?.toggle(event);
}

const isReserveSpecial = computed(() => {
  const { boardSize, pieces, capstones } = props.settings;
  return !isDefaultReserve(boardSize, { pieces, capstones });
});

const isOpeningSpecial = computed(() => {
  return props.settings.opening !== 'swap';
});

const isSpecial = computed(() => {
  return isReserveSpecial.value || isOpeningSpecial.value;
});

const openingNames: Record<string, string | undefined> = {
  swap: 'Swap',
  noSwap: 'No Swap',
  doubleStack: 'Double Stack',
};
</script>
<template>
  <Button v-if="isSpecial" class="w-8! h-8! p-1!" severity="warn" variant="text" @click="toggle">
    <template #icon>
      <LuInfo />
    </template>
  </Button>
  <Popover ref="detailsPopover">
    <div class="flex flex-col gap-1">
      <div v-if="isReserveSpecial" class="flex items-center gap-2 justify-start">
        <Fa6RegChessPawn class="text-primary" />
        {{ settings.pieces }} Flat{{ settings.pieces !== 1 ? 's' : '' }}
      </div>
      <div v-if="isReserveSpecial" class="flex items-center gap-2 justify-start">
        <Fa6RegChessQueen class="text-primary" />
        {{ settings.capstones }} Capstone{{ settings.capstones !== 1 ? 's' : '' }}
      </div>
      <div v-if="isOpeningSpecial" class="flex items-center gap-2 justify-start">
        <LuPlay class="text-primary" />
        {{ openingNames[settings.opening] }}
      </div>
    </div>
  </Popover>
</template>
