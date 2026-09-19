<script setup lang="ts">
import KratosFlow from '@/components/auth/KratosFlow.vue';
import { useAuthStore } from '@/features/auth';
import { Button } from '@tak-ui-lib/components';
import { useRouter } from 'vue-router';

const authStore = useAuthStore();
const router = useRouter();

async function onLogout() {
  await authStore.logout();
  await router.push('/login');
}
function onLogin() {
  void router.push('/login');
}
</script>

<template>
  <h1>Account Settings</h1>
  <div class="flex flex-col items-stretch gap-2">
    <KratosFlow v-if="authStore.authState.type === 'logged_in'" flow-type="settings" />
    <Button
      v-if="authStore.authState.type === 'logged_in'"
      label="Logout"
      severity="danger"
      @click="onLogout"
    />
    <Button v-else label="Go to Login" @click="onLogin" />
  </div>
</template>
