<script setup lang="ts">
import { useRefreshAccount, useRegister } from '@/api/auth';
import Page from '@/components/Page.vue';
import { useJwtStore } from '@/features/auth';
import { zodFormValidator } from '@/utils/forms';
import { Button, Form, InputText, useFormContext } from '@tak-ui-lib/components';
import { useRouter } from 'vue-router';
import z from 'zod';

const registerFormSchema = z.object({
  email: z.email(),
  username: z.string().min(1, 'Username is required'),
  password: z.string().min(1, 'Password is required'),
});
type RegisterForm = z.infer<typeof registerFormSchema>;

const formValidator = zodFormValidator(registerFormSchema);
const formContext = useFormContext({
  email: '',
  username: '',
  password: '',
});

const jwtStore = useJwtStore();
const register = useRegister();
const refreshAccount = useRefreshAccount();
const router = useRouter();

async function onSubmit(data: RegisterForm) {
  try {
    const { jwt } = await register(data);
    jwtStore.setJwt(jwt);
    await refreshAccount();
    await router.push('/');
  } catch (error) {
    console.error('Error during registration:', error);
  }
}
</script>

<template>
  <Page>
    <h1>Sign Up</h1>
    <div class="flex flex-col items-stretch gap-2">
      <Form v-model="formContext" :validator="formValidator" @submit="onSubmit">
        <div class="flex flex-col gap-2">
          <InputText label="Email" name="email" autocomplete="email" />
          <InputText label="Username" name="username" autocomplete="username" />
          <InputText label="Password" name="password" type="password" autocomplete="new-password" />
          <div class="flex justify-end">
            <Button type="submit">Sign Up</Button>
          </div>
        </div>
      </Form>
      <RouterLink to="/login" class="mt-4">Already have an account? Sign in here.</RouterLink>
    </div>
  </Page>
</template>
