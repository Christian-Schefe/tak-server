<script setup lang="ts" generic="T">
import { provideFormContext, type FormValidator } from '../../form';

const props = withDefaults(
  defineProps<{
    validator: FormValidator<T>;
    initialValues?: Record<string, unknown> | undefined;
  }>(),
  {
    initialValues: undefined,
  },
);

const emit = defineEmits<{
  submit: [data: T];
}>();

const formCtx = provideFormContext(() => props.initialValues);

function onSubmit(event: SubmitEvent) {
  if (event.submitter instanceof HTMLButtonElement && event.submitter.name !== '') {
    formCtx.value.data[event.submitter.name] = event.submitter.value;
  }
  const result = props.validator(formCtx.value.data);
  if (result.type === 'success') {
    formCtx.value.reset();
    emit('submit', result.data);
  } else {
    formCtx.value.errors = result.errors;
  }
}

function onReset() {
  formCtx.value.reset();
}
</script>
<template>
  <form @submit.prevent="onSubmit" @reset.prevent="onReset">
    <slot v-bind="formCtx" />
  </form>
</template>
