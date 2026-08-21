<script setup lang="ts"></script>
<template>
  <div class="p-card">
    <slot></slot>
  </div>
</template>
<style lang="css">
.p-card {
  background-color: var(--p-card-background);
  color: var(--p-card-text);
  border-radius: var(--p-card-border-radius);
  padding: var(--p-card-padding);
  gap: var(--p-card-gap);
  border: var(--p-card-border);
  box-shadow: var(--p-card-box-shadow);
  display: flex;
  flex-direction: column;
}
</style>
