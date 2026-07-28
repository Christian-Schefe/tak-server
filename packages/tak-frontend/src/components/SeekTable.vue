<script setup lang="ts">
import type { SeekInfo } from '@/api/seek';
import PlayerLabel from '@/components/PlayerLabel.vue';
import { timeControlToString } from '@/utils/time';
import Button from 'primevue/button';
import Column from 'primevue/column';
import DataTable from 'primevue/datatable';
import { computed } from 'vue';
import { LuSwords, LuTrash } from 'vue-icons-plus/lu';

type SeekEntry = {
  type: 'own' | 'other';
  seek: SeekInfo;
};

const props = defineProps<{
  seeks: SeekEntry[] | undefined;
}>();

defineEmits<{
  accept: [string];
  delete: [string];
}>();

const tableData = computed(() => {
  if (!props.seeks) return [];
  return props.seeks.map((seek) => {
    const boardSizeStr = seek.seek.gameSettings.boardSize.toString();
    return {
      id: seek.seek.id,
      action: seek.type === 'own' ? 'delete' : 'accept',
      creatorId: seek.seek.creatorId,
      boardSize: `${boardSizeStr}x${boardSizeStr}`,
      komi: seek.seek.gameSettings.halfKomi * 0.5,
      reserves: `${seek.seek.gameSettings.pieces.toString()}/${seek.seek.gameSettings.capstones.toString()}`,
      timeControl: timeControlToString(seek.seek.gameSettings.timeSettings),
    };
  });
});

const icons = {
  accept: LuSwords,
  delete: LuTrash,
};
</script>
<template>
  <DataTable :value="tableData" paginator :rows="10">
    <Column field="action">
      <template #body="slotProps">
        <Button
          v-if="slotProps.data.action === 'accept'"
          severity="secondary"
          @click="$emit('accept', slotProps.data.id)"
        >
          <template #icon><component :is="icons.accept"></component></template>
        </Button>
        <Button v-else severity="secondary" @click="$emit('delete', slotProps.data.id)">
          <template #icon><component :is="icons.delete"></component></template>
        </Button>
      </template>
    </Column>
    <Column header="Creator">
      <template #body="slotProps">
        <PlayerLabel :pid="slotProps.data.creatorId" type="player" />
      </template>
    </Column>
    <Column field="boardSize" header="Board Size"></Column>
    <Column field="komi" header="Komi"></Column>
    <Column field="reserves" header="Reserves"></Column>
    <Column field="timeControl" header="Time Control"></Column>
    <template #empty>
      <slot></slot>
    </template>
  </DataTable>
</template>
