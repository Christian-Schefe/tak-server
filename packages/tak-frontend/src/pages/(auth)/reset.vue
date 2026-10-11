<script setup lang="ts">
import { useReset } from '@/api/auth';
import Page from '@/components/Page.vue';
import { zodFormValidator } from '@/utils/forms';
import { Button, Form, InputText, useFormContext } from '@tak-ui-lib/components';
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import z from 'zod';

const resetFormSchema = z.object({
  identifier: z.string().min(1, 'Identifier is required'),
  newPassword: z.string().min(1, 'Password is required'),
  token: z.string().min(1, 'Token is required'),
});
type ResetForm = z.infer<typeof resetFormSchema>;

const formValidator = zodFormValidator(resetFormSchema);

const recover = useReset();
const router = useRouter();
const route = useRoute();

const tokenFromQuery = computed(() => {
  const token = route.query.token;
  return typeof token === 'string' && token.length > 0 ? decodeURIComponent(token) : null;
});

const formContext = useFormContext(() => {
  return {
    identifier: '',
    newPassword: '',
    token: tokenFromQuery.value,
  };
});

async function onSubmit(data: ResetForm) {
  try {
    await recover(data);
    await router.push('/login');
  } catch (error) {
    console.error('Error during reset:', error);
  }
}
</script>
<template>
  <Page>
    <h1>Reset Password</h1>
    <div class="flex flex-col items-stretch gap-2">
      <Form v-model="formContext" :validator="formValidator" @submit="onSubmit">
        <div class="flex flex-col gap-2">
          <InputText label="Identifier" name="identifier" autocomplete="username" />
          <InputText label="Token" name="token" />
          <InputText
            label="New Password"
            name="newPassword"
            type="password"
            autocomplete="new-password"
          />
          <div class="flex justify-end">
            <Button type="submit">Reset Password</Button>
          </div>
        </div>
      </Form>
    </div>
  </Page>
</template>
