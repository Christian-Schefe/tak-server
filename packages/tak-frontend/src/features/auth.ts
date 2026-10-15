import { useRefresh } from '@/api/auth';
import { StorageSerializers, useStorage } from '@vueuse/core';
import { jwtDecode } from 'jwt-decode';
import { defineStore } from 'pinia';
import { computed, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { z } from 'zod';

const authDataSchema = z.object({
  jwt: z.string(),
  refreshToken: z.string(),
  refreshTokenValidUntil: z.number(),
});

type AuthData = z.infer<typeof authDataSchema>;

function parseAuthData(data: unknown): AuthData | null {
  if (data === null) {
    return null;
  }
  const result = authDataSchema.safeParse(data);
  if (!result.success) {
    console.warn('Failed to parse auth data:', result.error);
    return null;
  }
  return result.data;
}

export const useAuthStore = defineStore('auth', () => {
  const storedAuthData = useStorage<AuthData | null>('jwt', null, undefined, {
    serializer: StorageSerializers.object,
  });
  const authData = ref<AuthData | null>(null);

  watch(
    storedAuthData,
    (newValue) => {
      console.log('Loaded stored auth data:', newValue);
      const parsed = parseAuthData(newValue);
      if (parsed && parsed.refreshTokenValidUntil > Date.now()) {
        setAuthData(parsed);
      } else {
        setAuthData(null);
      }
    },
    { immediate: true, once: true },
  );

  function setAuthData(newAuthData: AuthData | null) {
    console.log('Setting auth data:', newAuthData);
    authData.value = newAuthData;
    storedAuthData.value = newAuthData;
  }

  const data = computed(() => {
    if (authData.value === null) {
      return null;
    }
    try {
      const decoded = jwtDecode(authData.value.jwt);
      if (decoded.exp === undefined) {
        console.warn('JWT does not have a valid exp claim:', decoded);
        return null;
      }
      if (decoded.sub === undefined) {
        console.warn('JWT does not have a valid sub claim:', decoded);
        return null;
      }
      return { userId: decoded.sub, exp: decoded.exp, ...authData.value };
    } catch (error) {
      console.error('Failed to decode JWT:', error, authData.value);
      return null;
    }
  });

  const refresh = useRefresh();
  const router = useRouter();
  async function maybeRefreshToken() {
    if (data.value === null) {
      return;
    }
    const currentTime = Math.floor(Date.now() / 1000);
    const isCloseToExpiring = data.value.exp - currentTime <= 5 * 60; // 5 minutes
    if (!isCloseToExpiring) {
      return;
    }
    console.log('Refreshing token for user:', data.value.userId);
    try {
      const newAuthData = await refresh({
        userId: data.value.userId,
        refreshToken: data.value.refreshToken,
      });
      setAuthData(newAuthData);
    } catch (error) {
      console.error('Failed to refresh token:', error);
      setAuthData(null);
      void router.push({ name: '/(auth)/login' });
    }
  }

  return { setAuthData, data, maybeRefreshToken };
});
