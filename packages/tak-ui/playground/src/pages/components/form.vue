<script setup lang="ts">
import { Button, Form, InputText } from '@tak-ui-lib/components';
import type { FormValidator } from '@tak-ui-lib/components/src/form.ts';
import Page from '../../components/Page.vue';

interface FormData {
  name: string;
  email: string;
}

const validator: FormValidator<FormData> = (data: Record<string, unknown>) => {
  const errors: Record<string, string> = {};
  console.log('Validating form data:', data);
  if (typeof data.name !== 'string' || data.name.trim() === '') {
    errors.name = 'Name is required';
  }
  if (typeof data.email !== 'string' || data.email.trim() === '') {
    errors.email = 'Email is required';
  }
  if (Object.keys(errors).length > 0) {
    return { type: 'error', errors };
  }
  return { type: 'success', data: data as { name: string; email: string } };
};
</script>
<template>
  <Page>
    <h1>Forms</h1>
    <Form v-slot="form" :validator="validator" :initial-values="{ email: 'hi' }">
      <div class="flex flex-col gap-4">
        <InputText
          input-id="input1"
          name="name"
          label="Name"
          :support-text="form.errors.name"
          autocomplete="name"
        />
        <InputText
          input-id="input2"
          name="email"
          label="Email"
          :support-text="form.errors.email"
          autocomplete="email"
        />

        <div class="flex gap-2">
          <Button type="reset" label="Reset" />
          <Button type="submit" label="Submit" />
        </div>
      </div>
    </Form>
  </Page>
</template>
