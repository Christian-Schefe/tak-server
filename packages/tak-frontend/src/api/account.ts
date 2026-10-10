import { useSubscription } from '@/features/websocket';
import { useFetch } from '@/utils/fetch';
import { useQuery, useQueryClient } from '@tanstack/vue-query';
import { computed, toValue, type MaybeRefOrGetter } from 'vue';
import { z } from 'zod';

const usersOnlineSchema = z.object({
  userIds: z.array(z.string()),
});

export function useOnlineUsers() {
  const { fetchTyped } = useFetch();
  const queryClient = useQueryClient();
  useSubscription(['usersOnline'], 'usersOnline', usersOnlineSchema, (data) => {
    console.log('Received online users update', data);
    queryClient.setQueryData(['ws', 'usersOnline'], data.userIds);
  });
  return useQuery({
    queryKey: ['ws', 'usersOnline'],
    queryFn: async () => {
      return await fetchTyped(z.array(z.string()), '/api/accounts/online');
    },
  });
}

export function useIsUserOnline(userId: MaybeRefOrGetter<string | undefined>) {
  const onlineAccountsQuery = useOnlineUsers();
  return computed(() => {
    const onlineAccounts = onlineAccountsQuery.data.value;
    if (!onlineAccounts) {
      return undefined;
    }
    const resolvedUserId = toValue(userId);
    if (resolvedUserId === undefined) {
      return undefined;
    }
    return onlineAccounts.includes(resolvedUserId);
  });
}
