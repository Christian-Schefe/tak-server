<script setup lang="ts">
import KratosFlow from '@/components/auth/KratosFlow.vue';
import Page from '@/components/Page.vue';
import { useAuthStore } from '@/features/auth';

const authStore = useAuthStore();
</script>

<template>
  <Page>
    <h1>Sign Up</h1>
    <div class="flex flex-col items-stretch gap-2">
      <KratosFlow v-if="authStore.authState.type === 'logged_out'" flow-type="registration" />
      <p v-else>You are already logged in.</p>
      <RouterLink to="/login" class="mt-4 text-muted-color">
        Already have an account? Login here.
      </RouterLink>
    </div>
  </Page>
</template>
