import { useSubscription } from '@/features/websocket';
import type { TakPlayer } from '@/tak-core';
import { useFetch } from '@/utils/fetch';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { z } from 'zod';
import { gameSettingsSchema, type GameSettings } from './game';

export type SeekInfo = z.infer<typeof seekInfo>;

export interface CreateSeekPayload {
  color: TakPlayer | 'random';
  isRated: boolean;
  gameSettings: GameSettings;
}

const seekInfo = z.object({
  id: z.string(),
  creatorId: z.string(),
  color: z.enum(['white', 'black', 'random']),
  isRated: z.boolean(),
  gameSettings: gameSettingsSchema,
});

export function useSeeks() {
  const { fetchTyped } = useFetch();

  const queryClient = useQueryClient();
  useSubscription(
    ['seeks', 'seekCreated'],
    'seekCreated',
    z.object({ seek: seekInfo }),
    (newSeek) => {
      queryClient.setQueryData<SeekInfo[]>(['ws', 'seeks'], (oldData) =>
        oldData ? [...oldData, newSeek.seek] : [newSeek.seek],
      );
    },
  );
  useSubscription(
    ['seeks', 'seekRemoved'],
    'seekRemoved',
    z.object({ seekId: z.string() }),
    ({ seekId }) => {
      queryClient.setQueryData<SeekInfo[]>(['ws', 'seeks'], (oldData) =>
        oldData ? oldData.filter((seek) => seek.id !== seekId) : [],
      );
    },
  );

  return useQuery({
    queryKey: ['ws', 'seeks'],
    queryFn: async () => {
      return await fetchTyped(seekInfo.array(), '/api/seeks');
    },
  });
}

export function useCreateSeek() {
  const { fetchTyped } = useFetch();
  return useMutation({
    mutationFn: async (payload: CreateSeekPayload) => {
      return await fetchTyped(seekInfo, '/api/seeks', 'POST', payload);
    },
  });
}

export function useDeleteSeek() {
  const { fetch } = useFetch();
  return useMutation({
    mutationFn: async (seekId: string) => {
      await fetch(`/api/seeks/${seekId}`, 'DELETE');
    },
  });
}

export function useAcceptSeek() {
  const { fetch } = useFetch();
  return useMutation({
    mutationFn: async (seekId: string) => {
      await fetch(`/api/seeks/${seekId}/accept`, 'POST', {});
    },
  });
}
