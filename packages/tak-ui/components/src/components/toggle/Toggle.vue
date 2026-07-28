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
  <div class="p-toggle" :class="{ 'p-toggle-active': value, 'p-toggle-disabled': disabled }">
    <input
      :id="inputId"
      v-model="value"
      class="p-toggle-input"
      type="checkbox"
      :disabled="disabled"
    />
    <div class="p-toggle-handle" />
  </div>
</template>
<style lang="css" scoped>
.p-toggle {
  width: var(--p-toggle-width);
  height: var(--p-toggle-height);
  background-color: var(--p-toggle-background);
  border-radius: var(--p-toggle-height);
  transition: background-color 0.2s ease;
  position: relative;
}
.p-toggle:has(> .p-toggle-input:focus-visible) {
  outline: var(--p-toggle-focus-outline);
  outline-offset: var(--p-toggle-focus-outline-offset);
}
.p-toggle:hover {
  background-color: var(--p-toggle-hover-background);
}
.p-toggle.p-toggle-disabled {
  background-color: var(--p-toggle-disabled-background);
}
.p-toggle.p-toggle-active {
  background-color: var(--p-toggle-active-background);
}
.p-toggle.p-toggle-active:hover {
  background-color: var(--p-toggle-active-hover-background);
}
.p-toggle.p-toggle-disabled.p-toggle-active {
  background-color: var(--p-toggle-disabled-active-background);
}
.p-toggle-input {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  opacity: 0;
  z-index: 1;
  cursor: pointer;
}
.p-toggle.p-toggle-disabled .p-toggle-input {
  cursor: unset;
}
.p-toggle-handle {
  width: var(--p-toggle-handle-size);
  height: var(--p-toggle-handle-size);
  background-color: var(--p-toggle-handle-background);
  border-radius: var(--p-toggle-handle-size);
  border: var(--p-toggle-handle-border);
  top: 50%;
  left: calc(var(--p-toggle-height) / 2);
  position: absolute;
  transition:
    left 0.2s ease,
    background-color 0.2s ease;
  transform: translate(-50%, -50%);
}
.p-toggle:hover .p-toggle-handle {
  background-color: var(--p-toggle-handle-hover-background);
}
.p-toggle.p-toggle-disabled .p-toggle-handle {
  background-color: var(--p-toggle-handle-disabled-background);
}
.p-toggle.p-toggle-active .p-toggle-handle {
  left: calc(100% - var(--p-toggle-height) / 2);
  background-color: var(--p-toggle-handle-active-background);
}
.p-toggle.p-toggle-active:hover .p-toggle-handle {
  background-color: var(--p-toggle-handle-active-hover-background);
}
.p-toggle.p-toggle-disabled.p-toggle-active .p-toggle-handle {
  background-color: var(--p-toggle-handle-active-disabled-background);
}
</style>
