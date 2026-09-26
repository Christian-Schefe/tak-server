<script setup lang="ts">
import { triggerRef } from 'vue';
import { InputText } from '..';
import { useFormValue } from '../../form';

const model = defineModel<number | undefined>({ default: undefined });

const props = withDefaults(
  defineProps<{
    placeholder?: string | undefined;
    label?: string | undefined;
    inputId?: string | undefined;
    name?: string | undefined;
    mode?: 'decimal' | 'integer';
  }>(),
  {
    placeholder: undefined,
    label: undefined,
    inputId: undefined,
    name: undefined,
    mode: 'integer',
  },
);

function tryParseNumber(value: string): number | null {
  const parsedValue = props.mode === 'integer' ? parseInt(value, 10) : parseFloat(value);
  return isNaN(parsedValue) ? null : parsedValue;
}

function onTextChange(text: string) {
  if (text === '') {
    model.value = undefined;
    return;
  }
  const parsedValue = tryParseNumber(text);
  if (parsedValue !== null) {
    model.value = parsedValue;
  }
}
function onTextCommit(text: string) {
  const parsedValue = tryParseNumber(text);
  if (parsedValue !== null) {
    model.value = parsedValue;
  }
  triggerRef(model);
}
useFormValue(model, () => props.name);
</script>
<template>
  <InputText
    :model-value="model?.toString() ?? ''"
    :placeholder="placeholder"
    :label="label"
    :input-id="inputId"
    :name="name"
    disable-form-value
    @update:model-value="onTextChange"
    @change="onTextCommit"
  >
    <slot />
  </InputText>
</template>
