import z from 'zod';
import { gameMetadataSchema } from './game';
import { useFetch } from '@/utils/fetch';
import { useQuery } from '@tanstack/vue-query';
import { paginatedResponseSchema, type PaginationQuery } from '.';
import { toValue, type MaybeRefOrGetter } from 'vue';

export const gameHistoryRecordSchema = z.object({
  result: z.string().nullable(),
  ...gameMetadataSchema.shape,
});

export type GameHistoryRecord = z.infer<typeof gameHistoryRecordSchema>;

const gameHistorySchema = paginatedResponseSchema(gameHistoryRecordSchema);
export type GameHistory = z.infer<typeof gameHistorySchema>;

export function useGameHistory(pagination: MaybeRefOrGetter<PaginationQuery>) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['gameHistory', pagination],
    queryFn: async () => {
      const paginationValue = toValue(pagination);
      const params = new URLSearchParams({
        page: paginationValue.page.toString(),
        pageSize: paginationValue.pageSize.toString(),
      });
      return await fetchTyped(gameHistorySchema, `/api/history?${params.toString()}`);
    },
  });
}
