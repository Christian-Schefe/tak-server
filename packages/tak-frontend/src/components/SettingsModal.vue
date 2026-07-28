<script setup lang="ts">
import SettingsOptions from '@/components/SettingsOptions.vue';
import Dialog from 'primevue/dialog';

const visible = defineModel<boolean>({ required: true });
</script>
<template>
  <Dialog
    v-model:visible="visible"
    dismissable-mask
    header="Settings"
    :draggable="false"
    modal
    :style="{ width: '50vw', minHeight: '50vh' }"
    :breakpoints="{ '1199px': '75vw', '575px': '90vw' }"
  >
    <div class="max-w-xl mx-auto">
      <SettingsOptions />
    </div>
  </Dialog>
</template>
