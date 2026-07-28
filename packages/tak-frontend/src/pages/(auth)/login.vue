<script setup lang="ts">
import KratosFlow from '@/components/auth/KratosFlow.vue';
import { useAuthStore } from '@/features/auth';
import Button from 'primevue/button';
import Card from 'primevue/card';

const authStore = useAuthStore();

function onLogout() {
  void authStore.logout();
}
</script>

<template>
  <Card class="w-full max-w-lg mt-4">
    <template #title>Sign In</template>
    <template #content>
      <div class="flex flex-col items-stretch gap-2">
        <KratosFlow v-if="authStore.authState.type === 'logged_out'" flow-type="login" />
        <div v-else>
          <p>You are already logged in.</p>
          <Button @click="onLogout">Logout</Button>
        </div>
        <RouterLink to="/register" class="mt-4 text-muted-color">
          Don't have an account? Register here.
        </RouterLink>
      </div>
    </template>
  </Card>
</template>
