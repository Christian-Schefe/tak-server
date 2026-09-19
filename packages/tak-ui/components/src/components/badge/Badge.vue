<script setup lang="ts">
withDefaults(
  defineProps<{
    severity?: 'primary' | 'error';
    label?: string | undefined;
  }>(),
  {
    severity: 'primary',
    label: undefined,
  },
);
</script>
<template>
  <div :class="`p-badge p-badge-${severity}`">
    <span class="p-badge-label">{{ label }}</span>
    <slot />
  </div>
</template>
