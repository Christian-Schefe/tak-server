<script setup lang="ts">
import type { InputAutoCompleteAttribute, InputTypeHTMLAttribute } from 'vue';
import { LabelField } from '..';
import { useFormValue } from '../../form';

const model = defineModel<string>({ default: '' });
const props = withDefaults(
  defineProps<{
    name?: string | undefined;
    placeholder?: string | undefined;
    supportText?: string | undefined;
    label?: string | undefined;
    inputId?: string | undefined;
    disabled?: boolean;
    required?: boolean;
    hidden?: boolean;
    autocomplete?: InputAutoCompleteAttribute | undefined;
    type?: InputTypeHTMLAttribute | undefined;
  }>(),
  {
    name: undefined,
    placeholder: undefined,
    supportText: undefined,
    label: undefined,
    inputId: undefined,
    disabled: false,
    required: false,
    hidden: false,
    autocomplete: undefined,
    type: 'text',
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
  <LabelField
    :label="label"
    :label-for="inputId"
    :support-text="supportText"
    :disabled="disabled"
    :hidden="hidden"
  >
    <input
      :id="inputId"
      v-model="model"
      :name="name"
      class="p-inputtext"
      :placeholder="placeholder"
      :disabled="disabled"
      :required="required"
      :autocomplete="autocomplete"
      :hidden="hidden"
      :type="type"
      @change="onInputChange"
    />
    <template v-if="$slots['icon-prepend']" #icon-prepend>
      <slot name="icon-prepend" />
    </template>
    <template v-if="$slots['icon-append']" #icon-append>
      <slot name="icon-append" />
    </template>
  </LabelField>
</template>
