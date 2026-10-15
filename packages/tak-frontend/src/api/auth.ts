import { useAuthStore } from '@/features/auth';
import { useFetch } from '@/utils/fetch';
import { z } from 'zod';
import { usePlayerInfo } from './player';

export function useAccount() {
  const authStore = useAuthStore();
  return usePlayerInfo(() => authStore.data?.userId);
}

export interface LoginRequest {
  identifier: string;
  password: string;
}

export const authenticatedUserResponseSchema = z.object({
  jwt: z.string(),
  refreshToken: z.string(),
  refreshTokenValidUntil: z.number(),
});

export type AuthenticatedUserResponse = z.infer<typeof authenticatedUserResponseSchema>;

export function useLogin() {
  const { fetchTyped } = useFetch();
  return async (request: LoginRequest) => {
    return await fetchTyped(authenticatedUserResponseSchema, '/api/auth/login', 'POST', request);
  };
}

export function useLoginGuest() {
  const { fetchTyped } = useFetch();
  return async () => {
    return await fetchTyped(authenticatedUserResponseSchema, '/api/auth/login-guest', 'POST');
  };
}

export interface RegisterRequest {
  email: string;
  username: string;
  password: string;
}

export function useRegister() {
  const { fetchTyped } = useFetch();
  return async (request: RegisterRequest) => {
    return await fetchTyped(authenticatedUserResponseSchema, '/api/auth/register', 'POST', request);
  };
}

export interface RecoverRequest {
  identifier: string;
}

export function useRecover() {
  const { fetch } = useFetch();
  return async (request: RecoverRequest) => {
    return await fetch('/api/auth/recover', 'POST', request);
  };
}

export interface ResetRequest {
  identifier: string;
  token: string;
  newPassword: string;
}

export function useReset() {
  const { fetch } = useFetch();
  return async (request: ResetRequest) => {
    return await fetch('/api/auth/reset', 'POST', request);
  };
}

export interface RefreshRequest {
  userId: string;
  refreshToken: string;
}

export function useRefresh() {
  const { fetchTyped } = useFetch();
  return async (request: RefreshRequest) => {
    return await fetchTyped(authenticatedUserResponseSchema, '/api/auth/refresh', 'POST', request);
  };
}
