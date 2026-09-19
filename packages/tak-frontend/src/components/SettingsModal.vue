<script setup lang="ts">
import SettingsOptions from '@/components/SettingsOptions.vue';
import { Dialog } from '@tak-ui-lib/components';

const visible = defineModel<boolean>({ required: true });
</script>
<template>
  <Dialog v-model:visible="visible" header="Settings">
    <div class="max-w-xl mx-auto">
      <SettingsOptions />
    </div>
  </Dialog>
</template>
