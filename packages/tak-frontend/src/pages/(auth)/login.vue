<script setup lang="ts">
import { useLogin, useRefreshAccount } from '@/api/auth';
import Page from '@/components/Page.vue';
import { useJwtStore } from '@/features/auth';
import { zodFormValidator } from '@/utils/forms';
import { Button, Form, InputText, useFormContext } from '@tak-ui-lib/components';
import { useRouter } from 'vue-router';
import z from 'zod';

const loginFormSchema = z.object({
  identifier: z.string().min(1, 'Identifier is required'),
  password: z.string().min(1, 'Password is required'),
});
type LoginForm = z.infer<typeof loginFormSchema>;

const formValidator = zodFormValidator(loginFormSchema);
const formContext = useFormContext({
  identifier: '',
  password: '',
});

const jwtStore = useJwtStore();
const login = useLogin();
const refreshAccount = useRefreshAccount();
const router = useRouter();

async function onSubmit(data: LoginForm) {
  try {
    const { jwt } = await login(data);
    jwtStore.setJwt(jwt);
    await refreshAccount();
    await router.push('/');
  } catch (error) {
    console.error('Error during login:', error);
  }
}
</script>
<template>
  <Page>
    <h1>Sign In</h1>
    <div class="flex flex-col items-stretch gap-2">
      <Form v-model="formContext" :validator="formValidator" @submit="onSubmit">
        <div class="flex flex-col gap-2">
          <InputText label="Identifier" name="identifier" autocomplete="username" />
          <InputText
            label="Password"
            name="password"
            type="password"
            autocomplete="current-password"
          />
          <div class="flex justify-end">
            <Button type="submit">Sign In</Button>
          </div>
        </div>
      </Form>
      <RouterLink to="/register" class="mt-4"> Don't have an account? Sign up here. </RouterLink>
      <RouterLink to="/recover" class="mt-4"> Forgot your password? Recover it here. </RouterLink>
    </div>
  </Page>
</template>
