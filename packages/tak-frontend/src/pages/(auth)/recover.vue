<script setup lang="ts">
import KratosFlow from '@/components/auth/KratosFlow.vue';
import Card from 'primevue/card';
</script>

<template>
  <Card class="w-full max-w-lg mt-4">
    <template #title>Recover Account</template>
    <template #content>
      <div class="flex flex-col items-stretch gap-2">
        <KratosFlow flow-type="recovery" />
      </div>
    </template>
  </Card>
</template>
