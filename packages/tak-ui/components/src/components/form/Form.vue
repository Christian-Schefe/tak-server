<script setup lang="ts" generic="T">
import { provideFormContext, type FormValidator } from '../../form';

const props = withDefaults(
  defineProps<{
    validator: FormValidator<T>;
    initialValues?: Partial<T> | undefined;
  }>(),
  {
    initialValues: undefined,
  },
);

const emit = defineEmits<{
  submit: [data: T];
}>();

const { ctx: formCtx, resetForm } = provideFormContext(() => props.initialValues);

function onSubmit() {
  const result = props.validator(formCtx.value.data);
  if (result.type === 'success') {
    formCtx.value.errors = {};
    emit('submit', result.data);
  } else {
    formCtx.value.errors = result.errors;
  }
}

function onReset() {
  resetForm();
}
</script>
<template>
  <form @submit.prevent="onSubmit" @reset.prevent="onReset">
    <slot v-bind="formCtx" />
  </form>
</template>
