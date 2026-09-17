<script setup lang="ts">
const value = defineModel<boolean>({ default: false });
withDefaults(
  defineProps<{
    disabled?: boolean | undefined;
    inputId?: string | undefined;
  }>(),
  {
    disabled: false,
    inputId: undefined,
  },
);
</script>
<template>
  <div class="p-toggle" :class="{ 'p-toggle-on': value, 'p-toggle-disabled': disabled }">
    <input
      :id="inputId"
      v-model="value"
      class="p-toggle-input"
      type="checkbox"
      :disabled="disabled"
    />
    <div class="p-toggle-track" />
    <div class="p-toggle-handle" />
  </div>
</template>
