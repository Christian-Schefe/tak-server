<script setup lang="ts">
import { useRecover } from '@/api/auth';
import Page from '@/components/Page.vue';
import { zodFormValidator } from '@/utils/forms';
import { Button, Form, InputText, useFormContext } from '@tak-ui-lib/components';
import { useRouter } from 'vue-router';
import z from 'zod';

definePage({
  meta: {
    noAuth: true,
  },
});

const recoverFormSchema = z.object({
  identifier: z.string().min(1, 'Identifier is required'),
});
type RecoverForm = z.infer<typeof recoverFormSchema>;

const formValidator = zodFormValidator(recoverFormSchema);
const formContext = useFormContext({
  identifier: '',
});

const recover = useRecover();
const router = useRouter();

async function onSubmit(data: RecoverForm) {
  try {
    await recover(data);
    await router.push('/login');
  } catch (error) {
    console.error('Error during recover:', error);
  }
}
</script>
<template>
  <Page>
    <h1>Recover Account</h1>
    <div class="flex flex-col items-stretch gap-2">
      <Form v-model="formContext" :validator="formValidator" @submit="onSubmit">
        <div class="flex flex-col gap-4">
          <InputText label="Identifier" name="identifier" autocomplete="username" />
          <div class="flex justify-end">
            <Button type="submit">Recover Account</Button>
          </div>
        </div>
      </Form>
      <RouterLink to="/login" class="mt-4"> Didn't forget your password? Sign in. </RouterLink>
    </div>
  </Page>
</template>
