<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: 'filled' | 'elevated' | 'outlined';
  }>(),
  {
    variant: 'elevated',
  },
);
</script>
<template>
  <div class="p-card" :class="`p-card-${variant}`">
    <slot></slot>
  </div>
</template>
