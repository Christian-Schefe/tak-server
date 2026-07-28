import { useSubscription } from '@/features/websocket';
import { useFetch } from '@/utils/fetch';
import { useQuery, useQueryClient } from '@tanstack/vue-query';
import { computed, toValue, type MaybeRefOrGetter } from 'vue';
import { z } from 'zod';

const accountOnlineSchema = z.object({
  accountIds: z.array(z.string()),
});

export function useOnlineAccounts() {
  const { fetchTyped } = useFetch();
  const queryClient = useQueryClient();
  useSubscription(['accountsOnline'], 'accountsOnline', accountOnlineSchema, (data) => {
    console.log('Received online accounts update', data);
    queryClient.setQueryData(['ws', 'accountsOnline'], data.accountIds);
  });
  return useQuery({
    queryKey: ['ws', 'accountsOnline'],
    queryFn: async () => {
      return await fetchTyped(z.array(z.string()), '/api/accounts/online');
    },
  });
}

export function useIsAccountOnline(accountId: MaybeRefOrGetter<string | undefined>) {
  const onlineAccountsQuery = useOnlineAccounts();
  return computed(() => {
    const onlineAccounts = onlineAccountsQuery.data.value;
    if (!onlineAccounts) {
      return undefined;
    }
    const resolvedAccountId = toValue(accountId);
    if (resolvedAccountId === undefined) {
      return undefined;
    }
    return onlineAccounts.includes(resolvedAccountId);
  });
}
