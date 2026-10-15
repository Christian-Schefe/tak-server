import { useFetch } from '@/utils/fetch';
import { useQueries, useQuery } from '@tanstack/vue-query';
import { computed, type MaybeRefOrGetter, toValue } from 'vue';
import { z } from 'zod';
import { paginatedResponseSchema, type PaginationQuery } from '.';

const playerInfo = z.object({
  userId: z.string(),
  userRole: z.enum(['user', 'moderator', 'admin']),
  userType: z.enum(['player', 'guest', 'bot']),
  username: z.string(),
  participationRating: z.number().nullable(),
});

export function usePlayerInfo(userId: MaybeRefOrGetter<string | undefined>) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['playerInfo', userId],
    queryFn: async () => {
      const id = toValue(userId) ?? '';
      return await fetchTyped(playerInfo, `/api/players/player/${id}`);
    },
    enabled: computed(() => toValue(userId) !== undefined),
    staleTime: 1000 * 60 * 5,
  });
}

export function usePlayerInfos(userIds: MaybeRefOrGetter<string[] | undefined>) {
  const { fetchTyped } = useFetch();
  const queries = computed(() => {
    const ids = toValue(userIds) ?? [];
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
  userId: MaybeRefOrGetter<string | undefined>,
  from: MaybeRefOrGetter<number | undefined>,
  to: MaybeRefOrGetter<number | undefined>,
) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['ratingHistory', userId, from, to],
    queryFn: async () => {
      const userIdVal = toValue(userId);
      const fromVal = toValue(from);
      const toVal = toValue(to);
      if (userIdVal === undefined) {
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
        `/api/players/player/${userIdVal}/rating-history?${queryParams.toString()}`,
      );
    },
    staleTime: 1000 * 60 * 5,
    enabled: () => toValue(userId) !== undefined,
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

export function usePlayerStats(userId: MaybeRefOrGetter<string | undefined>) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['playerStats', userId],
    queryFn: async () => {
      const id = toValue(userId) ?? '';
      return await fetchTyped(playerStatsSchema, `/api/players/player/${id}/stats`);
    },
    staleTime: 1000 * 60 * 5,
    enabled: () => toValue(userId) !== undefined,
  });
}

export const playerLeaderboardSchema = paginatedResponseSchema(
  z.object({
    userId: z.string(),
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
