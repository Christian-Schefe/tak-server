<script setup lang="ts" generic="T extends object">
import { provide } from 'vue';
import { FormKey, type FormContext, type FormValidator } from '../../form';

const formCtx = defineModel<FormContext>({ required: true });

const props = defineProps<{
  validator: FormValidator<T>;
}>();

const emit = defineEmits<{
  submit: [data: T];
}>();

provide(FormKey, formCtx);

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
    <slot />
  </form>
</template>
