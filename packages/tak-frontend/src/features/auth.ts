import { useAccount } from '@/api/auth';
import { useStorage } from '@vueuse/core';
import { defineStore } from 'pinia';
import { ref, watch } from 'vue';

export const useJwtStore = defineStore('auth-jwt', () => {
  const storedJWT = useStorage<string | null>('jwt', null);
  const jwt = ref<string | null>(storedJWT.value);

  function setJwt(newJwt: string | null) {
    jwt.value = newJwt;
    storedJWT.value = newJwt;
  }

  return { jwt, setJwt };
});

export function useJwtSync() {
  const jwtStore = useJwtStore();
  const { data: account } = useAccount();
  watch(account, () => {
    if (account.value && account.value.jwt !== jwtStore.jwt) {
      jwtStore.setJwt(account.value.jwt);
    }
  });
}
