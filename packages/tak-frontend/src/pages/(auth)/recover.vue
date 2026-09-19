<script setup lang="ts">
import KratosFlow from '@/components/auth/KratosFlow.vue';
import Page from '@/components/Page.vue';
</script>

<template>
  <Page>
    <h1>Recover Account</h1>
    <div class="flex flex-col items-stretch gap-2">
      <KratosFlow flow-type="recovery" />
    </div>
  </Page>
</template>
