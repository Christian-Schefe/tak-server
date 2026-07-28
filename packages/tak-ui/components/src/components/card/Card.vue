<script setup lang="ts"></script>
<template>
  <div class="p-card">
    <slot></slot>
  </div>
</template>
<style lang="css">
.p-card {
  background-color: var(--p-card-background);
  border-radius: var(--p-card-border-radius);
  padding: var(--p-card-padding);
  gap: var(--p-card-gap);
  border: var(--p-card-border);
  display: flex;
  flex-direction: column;
}
</style>
