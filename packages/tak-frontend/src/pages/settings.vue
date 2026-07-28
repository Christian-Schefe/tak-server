<script setup lang="ts">
import SettingsOptions from '@/components/SettingsOptions.vue';
import Card from 'primevue/card';
</script>

<template>
  <div class="w-full mx-auto max-w-6xl p-4 flex flex-col gap-4">
    <Card class="w-full">
      <template #title>Settings</template>
      <template #content>
        <div class="max-w-xl mx-auto">
          <SettingsOptions />
        </div>
      </template>
    </Card>
  </div>
</template>
