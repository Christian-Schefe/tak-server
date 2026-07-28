import { useFetch } from '@/utils/fetch';
import { useQueries, useQuery } from '@tanstack/vue-query';
import { computed, type MaybeRefOrGetter, toValue } from 'vue';
import { z } from 'zod';
import { paginatedResponseSchema, type PaginationQuery } from '.';

const playerInfo = z.object({
  playerId: z.string(),
  accountId: z.string(),
  username: z.string(),
  displayName: z.string(),
  participationRating: z.number().nullable(),
});

export function usePlayerInfo(playerId: MaybeRefOrGetter<string | undefined>) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['playerInfo', playerId],
    queryFn: async () => {
      const id = toValue(playerId) ?? '';
      return await fetchTyped(playerInfo, `/api/players/player/${id}`);
    },
    enabled: computed(() => toValue(playerId) !== undefined),
    staleTime: 1000 * 60 * 5,
  });
}

export function usePlayerInfos(playerIds: MaybeRefOrGetter<string[] | undefined>) {
  const { fetchTyped } = useFetch();
  const queries = computed(() => {
    const ids = toValue(playerIds) ?? [];
    return ids.map((id) => ({
      queryKey: ['playerInfo', id],
      queryFn: async () => {
        return await fetchTyped(playerInfo, `/api/players/player/${id}`);
      },
      staleTime: 1000 * 60 * 5,
    }));
  });
  return useQueries({
    queries,
  });
}

export function useAccountOrPlayerInfo(
  id: MaybeRefOrGetter<string | undefined>,
  type: MaybeRefOrGetter<'account' | 'player'>,
) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['accountInfo', id, type],
    queryFn: async () => {
      const idVal = toValue(id) ?? '';
      const typeVal = toValue(type);
      if (typeVal === 'account') {
        return await fetchTyped(playerInfo, `/api/players/account/${idVal}`);
      } else {
        return await fetchTyped(playerInfo, `/api/players/player/${idVal}`);
      }
    },
    enabled: computed(() => toValue(id) !== undefined),
    staleTime: 1000 * 60 * 5,
  });
}

const ratingHistoryEntry = z.object({
  timestamp: z.number(),
  rating: z.number(),
});

const ratingHistoryResponse = z.object({
  entries: z.array(ratingHistoryEntry),
  firstEntryBeforeRange: ratingHistoryEntry.nullable(),
});
type RatingHistoryResponse = z.infer<typeof ratingHistoryResponse>;

export function useRatingHistory(
  playerId: MaybeRefOrGetter<string | undefined>,
  from: MaybeRefOrGetter<number | undefined>,
  to: MaybeRefOrGetter<number | undefined>,
) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['ratingHistory', playerId, from, to],
    queryFn: async () => {
      const playerIdVal = toValue(playerId);
      const fromVal = toValue(from);
      const toVal = toValue(to);
      if (playerIdVal === undefined) {
        return undefined;
      }
      const queryParams = new URLSearchParams();
      if (fromVal !== undefined) {
        queryParams.append('from', fromVal.toString());
      }
      if (toVal !== undefined) {
        queryParams.append('to', toVal.toString());
      }
      return await fetchTyped<RatingHistoryResponse>(
        ratingHistoryResponse,
        `/api/players/player/${playerIdVal}/rating-history?${queryParams.toString()}`,
      );
    },
    staleTime: 1000 * 60 * 5,
    enabled: () => toValue(playerId) !== undefined,
  });
}

const playerStatsSchema = z.object({
  ranking: z
    .object({
      rating: z.number(),
      maxRating: z.number(),
      rank: z.number(),
    })
    .nullable(),
  gamesPlayed: z.number(),
  ratedGamesPlayed: z.number(),
  gamesWon: z.number(),
  gamesLost: z.number(),
  gamesDrawn: z.number(),
  winStreak: z.number(),
  longestWinStreak: z.number(),
});

export type PlayerStats = z.infer<typeof playerStatsSchema>;

export function usePlayerStats(playerId: MaybeRefOrGetter<string | undefined>) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['playerStats', playerId],
    queryFn: async () => {
      const id = toValue(playerId) ?? '';
      return await fetchTyped(playerStatsSchema, `/api/players/player/${id}/stats`);
    },
    staleTime: 1000 * 60 * 5,
    enabled: () => toValue(playerId) !== undefined,
  });
}

export const playerLeaderboardSchema = paginatedResponseSchema(
  z.object({
    playerId: z.string(),
    rating: z.number(),
  }),
);

export function usePlayerLeaderboard(pagination: MaybeRefOrGetter<PaginationQuery>) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['playerLeaderboard', pagination],
    queryFn: async () => {
      const paginationValue = toValue(pagination);
      const params = new URLSearchParams({
        page: paginationValue.page.toString(),
        pageSize: paginationValue.pageSize.toString(),
      });
      return await fetchTyped(
        playerLeaderboardSchema,
        `/api/players/leaderboard?${params.toString()}`,
      );
    },
  });
}
