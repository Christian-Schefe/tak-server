<script setup lang="ts">
import { useRefreshAccount } from '@/api/auth';
import Page from '@/components/Page.vue';
import { useJwtStore } from '@/features/auth';
import { Button } from '@tak-ui-lib/components';
import { useRouter } from 'vue-router';

const jwtStore = useJwtStore();
const refreshAccount = useRefreshAccount();
const router = useRouter();

async function onLogout() {
  jwtStore.setJwt(null);
  await refreshAccount();
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
