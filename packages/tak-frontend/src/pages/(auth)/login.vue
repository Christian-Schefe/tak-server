<script setup lang="ts">
import KratosFlow from '@/components/auth/KratosFlow.vue';
import Page from '@/components/Page.vue';
import { useAuthStore } from '@/features/auth';
import { Button } from '@tak-ui-lib/components';

const authStore = useAuthStore();

function onLogout() {
  void authStore.logout();
}
</script>

<template>
  <Page>
    <h1>Sign In</h1>
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
  </Page>
</template>
