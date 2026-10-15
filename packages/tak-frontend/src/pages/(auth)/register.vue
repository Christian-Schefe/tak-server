<script setup lang="ts">
import { useRegister } from '@/api/auth';
import Page from '@/components/Page.vue';
import { useAuthStore } from '@/features/auth';
import { zodFormValidator } from '@/utils/forms';
import { Button, Form, InputText, useFormContext } from '@tak-ui-lib/components';
import { useRouter } from 'vue-router';
import z from 'zod';

definePage({
  meta: {
    noAuth: true,
  },
});

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

const authStore = useAuthStore();
const register = useRegister();
const router = useRouter();

async function onSubmit(data: RegisterForm) {
  try {
    const authData = await register(data);
    authStore.setAuthData(authData);
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
        <div class="flex flex-col gap-4">
          <InputText label="Username" name="username" />
          <InputText label="Email" name="email" autocomplete="username" />
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
