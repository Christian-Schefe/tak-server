<script setup lang="ts">
withDefaults(
  defineProps<{
    label?: string | undefined;
  }>(),
  {
    label: undefined,
  },
);
</script>
<template>
  <div
    :class="{
      'p-badge': true,
      'p-badge-with-label': label !== undefined,
    }"
  >
    <slot />
    <span class="p-badge-label">{{ label }}</span>
  </div>
</template>
