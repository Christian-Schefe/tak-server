import { useFetch } from '@/utils/fetch';
import { useQuery, useQueryClient } from '@tanstack/vue-query';
import { z } from 'zod';

export type AccountInfo = z.infer<typeof accountInfoSchema>;

const accountInfoSchema = z.object({
  userId: z.string(),
  isGuest: z.boolean(),
  isAdmin: z.boolean(),
  newGuest: z.boolean(),
  jwt: z.string(),
});

export const accountQueryKey = ['account'] as const;
export function useAccount() {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: accountQueryKey,
    queryFn: () => fetchTyped(accountInfoSchema, '/api/auth/whoami', 'GET', undefined, true, true),
    staleTime: Infinity,
  });
}

export function useRefreshAccount() {
  const queryClient = useQueryClient();
  return async function refreshAccount() {
    await queryClient.refetchQueries({ queryKey: accountQueryKey });
  };
}

export interface LoginRequest {
  identifier: string;
  password: string;
}

export const loginResponseSchema = z.object({
  jwt: z.string(),
});

export type LoginResponse = z.infer<typeof loginResponseSchema>;

export function useLogin() {
  const { fetchTyped } = useFetch();
  return async (request: LoginRequest) => {
    return await fetchTyped(loginResponseSchema, '/api/auth/login', 'POST', request);
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
    return await fetchTyped(loginResponseSchema, '/api/auth/register', 'POST', request);
  };
}
