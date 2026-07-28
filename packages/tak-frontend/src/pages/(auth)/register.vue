<script setup lang="ts">
import KratosFlow from '@/components/auth/KratosFlow.vue';
import { useAuthStore } from '@/features/auth';
import Card from 'primevue/card';

const authStore = useAuthStore();
</script>

<template>
  <Card class="w-full max-w-lg mt-4">
    <template #title>Sign Up</template>
    <template #content>
      <div class="flex flex-col items-stretch gap-2">
        <KratosFlow v-if="authStore.authState.type === 'logged_out'" flow-type="registration" />
        <p v-else>You are already logged in.</p>
        <RouterLink to="/login" class="mt-4 text-muted-color">
          Already have an account? Login here.
        </RouterLink>
      </div>
    </template>
  </Card>
</template>
