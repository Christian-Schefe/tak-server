<script setup lang="ts">
import KratosFlow from '@/components/auth/KratosFlow.vue';
import { useAuthStore } from '@/features/auth';
import Card from 'primevue/card';
import { useRouter } from 'vue-router';
import Button from 'primevue/button';

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
  <Card class="w-full max-w-lg mt-4">
    <template #title>Account Settings</template>
    <template #content>
      <div class="flex flex-col items-stretch gap-2">
        <KratosFlow v-if="authStore.authState.type === 'logged_in'" flow-type="settings" />
        <Button
          v-if="authStore.authState.type === 'logged_in'"
          label="Logout"
          severity="danger"
          @click="onLogout"
        /><Button v-else label="Go to Login" @click="onLogin" />
      </div>
    </template>
  </Card>
</template>
