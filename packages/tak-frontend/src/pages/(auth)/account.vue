<script setup lang="ts">
import Page from '@/components/Page.vue';
import { useAuthStore } from '@/features/auth';
import { Button } from '@tak-ui-lib/components';
import { useRouter } from 'vue-router';

const authStore = useAuthStore();
const router = useRouter();

async function onLogout() {
  authStore.setAuthData(null);
  await router.push('/login');
}
</script>

<template>
  <Page>
    <h1>Account Settings</h1>
    <div class="flex flex-col items-stretch gap-2">
      <Button label="Logout" severity="danger" @click="onLogout" />
    </div>
  </Page>
</template>
