import { useAccount } from '@/api/auth';
import { useSubscription, useWebSocketStore } from '@/features/websocket';
import { useFetch } from '@/utils/fetch';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { onUnmounted, ref, toValue, watch, type MaybeRefOrGetter } from 'vue';
import { useRouter } from 'vue-router';
import { z } from 'zod';

export const gameBaseSettingsSchema = z.object({
  boardSize: z.number(),
  halfKomi: z.number(),
  pieces: z.number(),
  capstones: z.number(),
  opening: z.enum(['swap', 'noSwap', 'doubleStack']),
});

export const gameSettingsSchema = z.object({
  ...gameBaseSettingsSchema.shape,
  timeSettings: z.union([
    z.object({
      type: z.literal('realtime'),
      contingentMs: z.number(),
      incrementMs: z.number(),
      extra: z
        .object({
          onMove: z.number(),
          extraMs: z.number(),
        })
        .nullable(),
    }),
    z.object({
      type: z.literal('async'),
      contingentMs: z.number(),
    }),
  ]),
});

export const gameEndedMessageSchema = z.object({
  gameId: z.string(),
  result: z.string(),
});

export const gameRequestsSchema = z.object({
  drawOffered: z.boolean(),
  undoRequested: z.boolean(),
  moreTimeOffered: z.number().nullable(),
});

export const gameStatusSchema = z.object({
  id: z.string(),
  matchId: z.string().nullable(),
  playerIds: z.object({
    white: z.string(),
    black: z.string(),
  }),
  isRated: z.boolean(),
  gameSettings: gameSettingsSchema,
  actions: z.array(z.string()),
  remainingMs: z.object({
    white: z.number(),
    black: z.number(),
  }),
  status: z.union([
    z.object({
      type: z.literal('ended'),
      result: z.enum(['1-0', '0-1', '1/2-1/2', '0-F', 'F-0', '0-R', 'R-0', '0-0']),
    }),
    z.object({
      type: z.literal('ongoing'),
      whiteRequests: gameRequestsSchema,
      blackRequests: gameRequestsSchema,
    }),
  ]),
});

export const gameRequestSchema = z.union([
  z.object({
    type: z.literal('draw'),
    offer: z.boolean(),
  }),
  z.object({
    type: z.literal('undo'),
    request: z.boolean(),
  }),
  z.object({
    type: z.literal('moreTime'),
    amountMs: z.number().nullable(),
  }),
]);

export type GameStatus = z.infer<typeof gameStatusSchema>;
export type GameSettings = z.infer<typeof gameSettingsSchema>;
export type GameMetadata = z.infer<typeof gameMetadataSchema>;
export type GameRequests = z.infer<typeof gameRequestsSchema>;
export type GameRequest = z.infer<typeof gameRequestSchema>;
export type GameRequestType = GameRequest['type'];

export const gameMetadataSchema = z.object({
  id: z.string(),
  date: z.number(),
  playerIds: z.object({
    white: z.string(),
    black: z.string(),
  }),
  isRated: z.boolean(),
  gameSettings: gameSettingsSchema,
});

export const timeInfo = z.object({ white: z.number(), black: z.number() });

export const gameEventBase = z.object({ gameId: z.string(), timeInfo });

export const gameEvent = z.union([
  z.object({
    eventType: z.literal('gameAction'),
    action: z.string(),
    plyIndex: z.number(),
    ...gameEventBase.shape,
  }),
  z.object({
    eventType: z.literal('gameActionUndone'),
    plyIndex: z.number(),
    ...gameEventBase.shape,
  }),
  z.object({
    eventType: z.literal('gameEnded'),
    result: z.enum(['1-0', '0-1', '1/2-1/2', '0-F', 'F-0', '0-R', 'R-0']),
    ...gameEventBase.shape,
  }),
  z.object({
    eventType: z.literal('gameRequestChanged'),
    request: gameRequestSchema,
    playerId: z.string(),
    ...gameEventBase.shape,
  }),
]);

export type GameEvent = z.infer<typeof gameEvent>;

export function useGameStatus(gameId: MaybeRefOrGetter<string>) {
  const { fetchTyped } = useFetch();

  const updateGameStatus = useUpdateGameStatus(gameId);
  useSubscription(['gameStatus', 'gameEvent', gameId], 'gameEvent', gameEvent, (event) => {
    const id = toValue(gameId);
    if (event.gameId === id) {
      updateGameStatus(event);
    }
  });

  return useQuery({
    queryKey: ['ws', 'gameStatus', gameId],
    queryFn: async () => {
      const id = toValue(gameId);
      return await fetchTyped(gameStatusSchema, `/api/games/${id}`);
    },
  });
}

const spectatingGames = new Map<string, number>();
function spectateGame(gameId: string) {
  const count = spectatingGames.get(gameId) ?? 0;
  spectatingGames.set(gameId, count + 1);
  return count === 0;
}

function unpectateGame(gameId: string) {
  const count = spectatingGames.get(gameId) ?? 0;
  if (count <= 1) {
    spectatingGames.delete(gameId);
    return true; // last subscriber left
  }
  spectatingGames.set(gameId, count - 1);
  return false;
}

export function useSpectateGame(gameId: MaybeRefOrGetter<string | undefined>) {
  const websocketStore = useWebSocketStore();
  const currentSubscription = ref<string | null>(null);

  watch(
    [() => websocketStore.isAuthenticated, () => toValue(gameId)],
    ([newIsAuthenticated, newGameId]) => {
      if (!newIsAuthenticated) {
        spectatingGames.clear();
        currentSubscription.value = null;
        return;
      }
      if (currentSubscription.value !== null && newGameId !== currentSubscription.value) {
        const shouldLeaveGame = unpectateGame(currentSubscription.value);
        currentSubscription.value = null;
        if (shouldLeaveGame) {
          console.log('Unsubscribing from spectate game with id', currentSubscription.value);
          void websocketStore.sendMessage({
            type: 'spectateGame',
            gameId: currentSubscription.value,
            spectate: false,
          });
        }
      }
      if (
        newGameId !== undefined &&
        (currentSubscription.value === null || newGameId !== currentSubscription.value)
      ) {
        const shouldSpectateGame = spectateGame(newGameId);
        currentSubscription.value = newGameId;
        if (shouldSpectateGame) {
          console.log('Subscribing to spectate game with id', newGameId);
          void websocketStore.sendMessage({
            type: 'spectateGame',
            gameId: newGameId,
            spectate: true,
          });
        }
      }
    },
    { immediate: true },
  );
  onUnmounted(() => {
    if (currentSubscription.value === null) return;
    const shouldLeaveGame = unpectateGame(currentSubscription.value);
    if (shouldLeaveGame) {
      console.log('Unsubscribing from spectate game with id', currentSubscription.value);
      void websocketStore.sendMessage({
        type: 'spectateGame',
        gameId: currentSubscription.value,
        spectate: false,
      });
    }
  });
}

export function useUpdateGameStatus(gameId: MaybeRefOrGetter<string>) {
  const queryClient = useQueryClient();
  const updateGameStatus = (event: GameEvent) => {
    const id = toValue(gameId);
    queryClient.setQueryData<GameStatus>(['ws', 'gameStatus', id], (oldStatus) => {
      if (!oldStatus) {
        void queryClient.invalidateQueries({ queryKey: ['ws', 'gameStatus', id] });
        console.warn('Received game event for game status that is not in cache, refetching...');
        return oldStatus;
      }

      const newStatus = computeNewGameStatus(oldStatus, event);
      if (!newStatus) {
        void queryClient.invalidateQueries({ queryKey: ['ws', 'gameStatus', id] });
        console.warn('Game status out of sync, refetching...');
        return oldStatus;
      }
      return newStatus;
    });
  };
  return updateGameStatus;
}

function computeNewGameStatus(oldStatus: GameStatus, event: GameEvent): GameStatus | null {
  switch (event.eventType) {
    case 'gameAction': {
      const resultingPlyIndex = oldStatus.actions.length + 1;
      if (resultingPlyIndex === event.plyIndex) {
        return {
          ...oldStatus,
          actions: [...oldStatus.actions, event.action],
          remainingMs: event.timeInfo,
        };
      } else if (
        resultingPlyIndex - 1 === event.plyIndex &&
        oldStatus.actions[oldStatus.actions.length - 1] === event.action
      ) {
        // This can happen when we optimistically apply an action and then receive the event for it, in which case we don't want to refetch
        return {
          ...oldStatus,
          remainingMs: event.timeInfo,
        };
      }
      return null;
    }
    case 'gameActionUndone': {
      const resultingPlyIndex = oldStatus.actions.length - 1;
      if (resultingPlyIndex !== event.plyIndex) {
        return null;
      }
      return {
        ...oldStatus,
        actions: oldStatus.actions.slice(0, -1),
        remainingMs: event.timeInfo,
      };
    }
    case 'gameEnded':
      return {
        ...oldStatus,
        status: { type: 'ended', result: event.result },
        remainingMs: event.timeInfo,
      };
    case 'gameRequestChanged':
      if (oldStatus.status.type !== 'ongoing') return oldStatus;
      return {
        ...oldStatus,
        remainingMs: event.timeInfo,
        status: {
          ...oldStatus.status,
          whiteRequests:
            event.playerId === oldStatus.playerIds.white
              ? {
                  ...oldStatus.status.whiteRequests,
                  ...(event.request.type === 'draw' && { drawOffered: event.request.offer }),
                  ...(event.request.type === 'undo' && { undoRequested: event.request.request }),
                  ...(event.request.type === 'moreTime' && {
                    moreTimeOffered: event.request.amountMs,
                  }),
                }
              : oldStatus.status.whiteRequests,
          blackRequests:
            event.playerId === oldStatus.playerIds.black
              ? {
                  ...oldStatus.status.blackRequests,
                  ...(event.request.type === 'draw' && { drawOffered: event.request.offer }),
                  ...(event.request.type === 'undo' && { undoRequested: event.request.request }),
                  ...(event.request.type === 'moreTime' && {
                    moreTimeOffered: event.request.amountMs,
                  }),
                }
              : oldStatus.status.blackRequests,
        },
      };
  }
}

export function useGames() {
  const { fetchTyped } = useFetch();

  const queryClient = useQueryClient();
  useSubscription(
    ['games', 'gameStarted'],
    'gameStarted',
    z.object({ game: gameMetadataSchema }),
    ({ game }) => {
      queryClient.setQueryData<GameMetadata[]>(['ws', 'games'], (oldData) =>
        oldData ? [...oldData, game] : [game],
      );
    },
  );
  useSubscription(
    ['games', 'gameEnded'],
    'gameEnded',
    z.object({ gameId: z.string() }),
    ({ gameId }) => {
      queryClient.setQueryData<GameMetadata[]>(['ws', 'games'], (oldData) =>
        oldData ? oldData.filter((game) => game.id !== gameId) : [],
      );
    },
  );

  return useQuery({
    queryKey: ['ws', 'games'],
    queryFn: async () => {
      return await fetchTyped(gameMetadataSchema.array(), '/api/games');
    },
  });
}

export function useNavigateOnGamesStartWebSocketSubscription() {
  const { data: account } = useAccount();
  const router = useRouter();
  useSubscription(
    ['gameNavigation', 'gameStarted'],
    'gameStarted',
    z.object({ game: gameMetadataSchema }),
    ({ game }) => {
      const isOwnGame =
        game.playerIds.white === account.value?.playerId ||
        game.playerIds.black === account.value?.playerId;
      if (isOwnGame) {
        void router.push(`/online/${game.id}`);
      }
    },
  );
}

export function useSetGameRequest(gameId: MaybeRefOrGetter<string>) {
  const { fetch } = useFetch();
  return useMutation({
    mutationFn: async (request: GameRequest) => {
      const id = toValue(gameId);
      await fetch(`/api/games/${id}/request`, 'POST', request);
    },
  });
}

export function useAcceptGameRequest(gameId: MaybeRefOrGetter<string>) {
  const { fetch } = useFetch();
  return useMutation({
    mutationFn: async (requestType: GameRequestType) => {
      const id = toValue(gameId);
      await fetch(`/api/games/${id}/request/accept`, 'POST', { type: requestType });
    },
  });
}
