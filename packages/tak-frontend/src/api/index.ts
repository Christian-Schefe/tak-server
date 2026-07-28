import z from 'zod';

export function paginatedResponseSchema<T>(item: z.ZodType<T>) {
  return z.object({
    items: z.array(item),
    totalCount: z.number(),
  });
}

export type PaginationQuery = {
  page: number;
  pageSize: number;
};
