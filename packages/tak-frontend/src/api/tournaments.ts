import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { z } from 'zod';
import { toValue, type MaybeRefOrGetter } from 'vue';
import { useFetch } from '@/utils/fetch';
import { matchSettingsSchema } from './match';

const tournamentMetadataSchema = z.object({
  id: z.string(),
  name: z.string(),
  matchSettings: matchSettingsSchema,
  tournamentFormat: z.union([
    z.object({ type: z.literal('swiss'), rounds: z.number() }),
    z.object({ type: z.literal('roundRobin') }),
  ]),
});
const tournamentSchema = z.object({
  metadata: tournamentMetadataSchema,
  status: z.union([
    z.object({ type: z.literal('upcoming'), registrationOpen: z.boolean() }),
    z.object({ type: z.literal('ongoing') }),
    z.object({ type: z.literal('completed') }),
  ]),
});

const tournamentDetailSchema = z.object({
  ...tournamentSchema.shape,
  players: z.array(
    z.object({
      id: z.string(),
      score: z.number(),
    }),
  ),
  rounds: z.array(
    z.object({
      matches: z.array(z.string()),
      byes: z.array(z.string()),
    }),
  ),
});

export type TournamentMetadata = z.infer<typeof tournamentMetadataSchema>;
export type Tournament = z.infer<typeof tournamentSchema>;

export type CreateTournamentData = Omit<TournamentMetadata, 'id'>;

export function useTournaments() {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['tournaments'],
    queryFn: async () => {
      return await fetchTyped(z.array(tournamentSchema), '/api/tournaments');
    },
  });
}

export function useTournament(id: MaybeRefOrGetter<string>) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['tournament', id],
    queryFn: async () => {
      return await fetchTyped(tournamentDetailSchema, `/api/tournaments/${toValue(id)}`);
    },
  });
}

export function useCreateTournament() {
  const queryClient = useQueryClient();
  const { fetch } = useFetch();
  return useMutation({
    mutationFn: async (data: CreateTournamentData) => {
      await fetch('/api/tournaments', 'POST', data);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['tournaments'] });
    },
  });
}

export function useStartTournament() {
  const queryClient = useQueryClient();
  const { fetch } = useFetch();
  return useMutation({
    mutationFn: async (tournamentId: string) => {
      await fetch(`/api/tournaments/${tournamentId}/start`, 'POST', {});
      return tournamentId;
    },
    onSuccess: async (tournamentId: string) => {
      await queryClient.invalidateQueries({ queryKey: ['tournament', tournamentId] });
    },
  });
}

export function useFinishTournament() {
  const queryClient = useQueryClient();
  const { fetch } = useFetch();
  return useMutation({
    mutationFn: async (tournamentId: string) => {
      await fetch(`/api/tournaments/${tournamentId}/finish`, 'POST', {});
      return tournamentId;
    },
    onSuccess: async (tournamentId: string) => {
      await queryClient.invalidateQueries({ queryKey: ['tournament', tournamentId] });
    },
  });
}

export function useStartNextRound() {
  const queryClient = useQueryClient();
  const { fetch } = useFetch();
  return useMutation({
    mutationFn: async (tournamentId: string) => {
      await fetch(`/api/tournaments/${tournamentId}/next-round`, 'POST', {});
      return tournamentId;
    },
    onSuccess: async (tournamentId: string) => {
      await queryClient.invalidateQueries({ queryKey: ['tournament', tournamentId] });
    },
  });
}

export function useRegisterForTournament() {
  const queryClient = useQueryClient();
  const { fetch } = useFetch();
  return useMutation({
    mutationFn: async (tournamentId: string) => {
      await fetch(`/api/tournaments/${tournamentId}/players`, 'POST', {});
      return tournamentId;
    },
    onSuccess: async (tournamentId: string) => {
      await queryClient.invalidateQueries({ queryKey: ['tournament', tournamentId] });
    },
  });
}

export function useDeregisterFromTournament() {
  const queryClient = useQueryClient();
  const { fetch } = useFetch();
  return useMutation({
    mutationFn: async (tournamentId: string) => {
      await fetch(`/api/tournaments/${tournamentId}/players`, 'DELETE');
      return tournamentId;
    },
    onSuccess: async (tournamentId: string) => {
      await queryClient.invalidateQueries({ queryKey: ['tournament', tournamentId] });
    },
  });
}

export function useSetRegistrationOpen() {
  const queryClient = useQueryClient();
  const { fetch } = useFetch();
  return useMutation({
    mutationFn: async ({ tournamentId, open }: { tournamentId: string; open: boolean }) => {
      await fetch(`/api/tournaments/${tournamentId}/registration`, 'POST', {
        registrationOpen: open,
      });
      return tournamentId;
    },
    onSuccess: async (tournamentId: string) => {
      await queryClient.invalidateQueries({ queryKey: ['tournament', tournamentId] });
    },
  });
}
