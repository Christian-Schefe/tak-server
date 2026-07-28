<script setup lang="ts">
import { triggerRef } from 'vue';
import { InputText } from '../inputtext';

const model = defineModel<number>({ default: 0 });
withDefaults(
  defineProps<{
    placeholder?: string | undefined;
    label?: string | undefined;
    inputId?: string | undefined;
  }>(),
  {
    placeholder: undefined,
    label: undefined,
    inputId: undefined,
  },
);

function isPartialNumber(value: string): boolean {
  return value === '' || value === '-' || value === '.' || value === '-.';
}
function tryParseFloat(value: string): number | null {
  const parsedValue = parseFloat(value);
  return isNaN(parsedValue) ? null : parsedValue;
}

function onTextChange(text: string) {
  if (isPartialNumber(text)) {
    return;
  }
  const parsedValue = tryParseFloat(text);
  if (parsedValue !== null) {
    model.value = parsedValue;
  }
}
function onTextCommit(text: string) {
  const newValue = tryParseFloat(text) ?? 0;
  model.value = newValue;
  triggerRef(model);
}
</script>
<template>
  <InputText
    :model-value="model.toString()"
    :placeholder="placeholder"
    :label="label"
    :input-id="inputId"
    @update:model-value="onTextChange"
    @change="onTextCommit"
  >
    <slot />
  </InputText>
</template>
