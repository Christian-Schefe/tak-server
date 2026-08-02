<script setup lang="ts">
import { useFormValue } from '../../form';

const model = defineModel<string>({ default: '' });
const props = withDefaults(
  defineProps<{
    name?: string | undefined;
    placeholder?: string | undefined;
    label?: string | undefined;
    inputId?: string | undefined;
  }>(),
  {
    name: undefined,
    placeholder: undefined,
    label: undefined,
    inputId: undefined,
  },
);
const emit = defineEmits<{
  change: [string];
}>();

function onInputChange(event: Event) {
  const target = event.target as HTMLInputElement;
  model.value = target.value;
  emit('change', target.value);
}

useFormValue(model, () => props.name);
</script>
<template>
  <div class="p-inputtext-container">
    <div class="p-inputtext-container-inner">
      <input
        :id="inputId"
        v-model="model"
        :name="name"
        class="p-inputtext"
        :class="{ 'p-inputtext-with-label': !!label }"
        :placeholder="placeholder"
        @change="onInputChange"
      />
      <label v-if="label" class="p-inputtext-label" :for="inputId">{{ label }}</label>
    </div>
    <slot />
  </div>
</template>
<style lang="css">
.p-inputtext-container {
  background-color: var(--p-inputtext-background);
  border: var(--p-inputtext-border);
  border-radius: var(--p-inputtext-border-radius);
  margin: 0;
  transition: border 0.2s ease-in-out;
  display: flex;
  color: var(--p-inputtext-empty-text);
  width: var(--p-inputtext-width);
  padding-left: var(--p-inputtext-padding-x);
  padding-right: var(--p-inputtext-padding-x);
  gap: var(--p-inputtext-padding-x);
  max-width: 100%;
  font-size: var(--p-inputtext-font-size);
  line-height: var(--p-inputtext-line-height);
  position: relative;
  align-items: center;
}
.p-inputtext-container-inner {
  display: flex;
  flex-direction: column;
  flex-grow: 1;
}
.p-inputtext {
  width: 100%;
  border: none;
  outline: none;
  margin: 0;
  color: var(--p-inputtext-filled-text);
  padding-top: var(--p-inputtext-padding-y);
  padding-bottom: var(--p-inputtext-padding-y);
}
.p-inputtext.p-inputtext-with-label {
  padding-top: calc(var(--p-inputtext-padding-y) + 0.625rem);
}
.p-inputtext::placeholder {
  color: var(--p-inputtext-empty-text);
}
.p-inputtext-label {
  font-size: 0.625rem;
  line-height: 1;
  transition: color 0.2s ease-in-out;
  position: absolute;
  top: var(--p-inputtext-padding-y);
  left: var(--p-inputtext-padding-x);
}
.p-inputtext:focus + .p-inputtext-label {
  color: var(--p-inputtext-label-text-focus);
}
.p-inputtext-container:hover {
  border: var(--p-inputtext-border-hover);
}
.p-inputtext-container:focus-within {
  border: var(--p-inputtext-border-focus);
}
</style>
