<script setup lang="ts">
withDefaults(
  defineProps<{
    label?: string | undefined;
    severity?: 'primary' | 'secondary' | 'tertiary' | 'danger';
  }>(),
  {
    label: undefined,
    severity: 'primary',
  },
);
</script>
<template>
  <div :class="`p-tag p-tag-${severity}`">
    <slot>{{ label }}</slot>
  </div>
</template>
