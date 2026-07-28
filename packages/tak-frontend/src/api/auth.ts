import { useFetch } from '@/utils/fetch';
import { useQuery, useQueryClient } from '@tanstack/vue-query';
import { z } from 'zod';

export type AccountInfo = z.infer<typeof accountInfoSchema>;

const accountInfoSchema = z.object({
  accountId: z.string(),
  playerId: z.string(),
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
