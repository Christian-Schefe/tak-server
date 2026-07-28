import { useMutation, useQueries, useQuery, useQueryClient } from '@tanstack/vue-query';
import { computed, toValue, type MaybeRefOrGetter } from 'vue';
import z from 'zod';
import { gameSettingsSchema } from './game';
import { useSubscription } from '@/features/websocket';
import { gameHistoryRecordSchema } from './gameHistory';
import { useFetch } from '@/utils/fetch';

const matchPlayerSchema = z.object({
  playerId: z.string(),
  score: z.number(),
});

export const matchSettingsSchema = z.object({
  gameSettings: gameSettingsSchema,
  matchMode: z.union([
    z.object({ type: z.literal('unlimited') }),
    z.object({ type: z.literal('fixedGames'), games: z.number() }),
    z.object({ type: z.literal('firstTo'), score: z.number() }),
  ]),
  isRated: z.boolean(),
});

export const matchDetailSchema = z.object({
  id: z.string(),
  player1: matchPlayerSchema,
  player2: matchPlayerSchema,
  settings: matchSettingsSchema,
  status: z.enum(['waiting', 'ongoing', 'completed']),
});

export const matchEvent = z.union([
  z.object({
    eventType: z.literal('readinessChanged'),
    matchId: z.string(),
    playerId: z.string().nullable(),
  }),
]);

export type MatchDetail = z.infer<typeof matchDetailSchema>;

export const matchReadinessSchema = z.object({
  readyPlayer: z.string().nullable(),
});

export type MatchReadiness = z.infer<typeof matchReadinessSchema>;

export function useMatch(id: MaybeRefOrGetter<string>) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['match', id],
    queryFn: async () => {
      return await fetchTyped(matchDetailSchema, `/api/matches/${toValue(id)}`);
    },
  });
}

export function useMatchGames(id: MaybeRefOrGetter<string>) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['matchGames', id],
    queryFn: async () => {
      return await fetchTyped(
        z.array(gameHistoryRecordSchema),
        `/api/matches/${toValue(id)}/games`,
      );
    },
  });
}

export function useMatches(ids: MaybeRefOrGetter<string[]>) {
  const { fetchTyped } = useFetch();
  const queries = computed(() => {
    const matchIds = toValue(ids);
    return matchIds.map((id) => ({
      queryKey: ['match', id],
      queryFn: async () => {
        return await fetchTyped(matchDetailSchema, `/api/matches/${id}`);
      },
    }));
  });
  return useQueries({
    queries,
    combine(result) {
      return Object.fromEntries(result.map((res) => [res.data?.id ?? '', res.data]));
    },
  });
}

export function useMatchReadiness(
  matchId: MaybeRefOrGetter<string>,
  enabled: MaybeRefOrGetter<boolean> = true,
) {
  const { fetchTyped } = useFetch();

  const queryClient = useQueryClient();
  useSubscription(['matchReadiness', 'matchEvent'], 'matchEvent', matchEvent, (event) => {
    const id = toValue(matchId);
    if (event.matchId === id) {
      queryClient.setQueryData<MatchReadiness>(['ws', 'matchReadiness', id], (oldMatch) => {
        if (!oldMatch) {
          void queryClient.invalidateQueries({ queryKey: ['ws', 'matchReadiness', id] });
          return oldMatch;
        }

        return {
          ...oldMatch,
          readyPlayer: event.playerId,
        };
      });
    }
  });

  return useQuery({
    queryKey: ['ws', 'matchReadiness', matchId],
    queryFn: async () => {
      return await fetchTyped(matchReadinessSchema, `/api/matches/${toValue(matchId)}/readiness`);
    },
    enabled,
  });
}

export function useMatchSetPlayerReady() {
  const queryClient = useQueryClient();
  const { fetch } = useFetch();
  return useMutation({
    mutationFn: async ({ id, ready }: { id: string; ready: boolean }) => {
      if (ready) {
        await fetch(`/api/matches/${id}/readiness`, 'POST', {});
      } else {
        await fetch(`/api/matches/${id}/readiness`, 'DELETE');
      }
      return id;
    },
    onSuccess: async (id: string) => {
      await queryClient.invalidateQueries({ queryKey: ['ws', 'matchReadiness', id] });
    },
  });
}
