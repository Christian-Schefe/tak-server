import { useWebSocket } from '@vueuse/core';
import { defineStore } from 'pinia';
import { computed, onUnmounted, ref, toValue, watch, type MaybeRefOrGetter } from 'vue';
import z from 'zod';
import { useAccount } from '@/api/auth';
import { useQueryClient } from '@tanstack/vue-query';

interface WsSubscription {
  type: string;
  parser: z.ZodType;
  subscriptionCount: number;
  handler: (msg: unknown) => void;
}

export type SubscriptionId = MaybeRefOrGetter<string | number | boolean>[];

const msgParser = z.union([
  z.object({
    type: z.literal('success'),
    responseId: z.string(),
  }),
  z.object({
    type: z.literal('error'),
    responseId: z.string(),
    message: z.string(),
    code: z.number(),
  }),
]);

const otherMsgParser = z.object({
  type: z.string(),
});

const inFlightMessages = new Map<
  string,
  (response: { type: 'success' } | { type: 'error'; message: string }) => void
>();

const subscriptions = new Map<string, WsSubscription>();

export const useWebSocketStore = defineStore('websocket', () => {
  function subscribe<T>(
    key: string,
    type: string,
    parser: z.ZodType<T>,
    handler: (msg: T) => void,
  ): () => void {
    const currentSub = subscriptions.get(key);
    if (currentSub) {
      if (currentSub.type !== type) {
        throw new Error(
          `Subscription with key ${key} already exists with different type ${currentSub.type}`,
        );
      }
      currentSub.subscriptionCount++;
    } else {
      subscriptions.set(key, {
        type,
        parser,
        subscriptionCount: 1,
        handler: handler as (msg: unknown) => void,
      });
      console.log('WebSocket subscription added:', key);
    }
    return () => {
      const sub = subscriptions.get(key);
      if (sub) {
        sub.subscriptionCount--;
        if (sub.subscriptionCount === 0) {
          subscriptions.delete(key);
          console.log('WebSocket subscription removed:', key);
        }
      }
    };
  }

  const wsUrl = import.meta.env.VITE_WS_URL;
  console.log('Connecting to WebSocket at', wsUrl);

  const { data, send, status } = useWebSocket(wsUrl, {
    autoReconnect: true,
  });

  async function sendMessage(message: object): Promise<void> {
    const messageId = crypto.randomUUID();
    const messageWithId = { responseId: messageId, ...message };
    send(JSON.stringify(messageWithId));
    console.log('WebSocket message sent:', messageWithId);
    return new Promise<void>((resolve, reject) => {
      inFlightMessages.set(messageId, (value) => {
        if (value.type === 'success') {
          resolve();
        } else {
          reject(new Error(value.message));
        }
      });
    });
  }

  watch(data, (newData) => {
    console.log('WebSocket data:', newData);
    if (typeof newData !== 'string') {
      return;
    }
    let value: unknown;
    try {
      value = JSON.parse(newData);
    } catch (error) {
      console.error('Failed to parse WebSocket message as JSON:', error);
      return;
    }
    console.log('WebSocket message received:', value);
    const parsed = msgParser.safeParse(value);
    if (parsed.success) {
      const responseId = parsed.data.responseId;
      const resolve = inFlightMessages.get(responseId);
      inFlightMessages.delete(responseId);

      if (resolve) {
        if (parsed.data.type === 'success') {
          resolve({ type: 'success' });
        } else {
          resolve({ type: 'error', message: parsed.data.message });
        }
      } else {
        console.error('No subscriber found for responseId:', responseId);
      }
    }

    const otherParsed = otherMsgParser.safeParse(value);
    if (!otherParsed.success) {
      console.error('WebSocket message parsing failed:', value, otherParsed.error);
      return;
    }

    subscriptions.forEach((sub) => {
      if (sub.type === otherParsed.data.type) {
        const parsed = sub.parser.safeParse(value);
        if (!parsed.success) {
          console.error('WebSocket message parsing failed:', value, parsed.error);
          return;
        }
        sub.handler(parsed.data);
      }
    });
  });

  const { data: account } = useAccount();

  const lastAuthenticated = ref<string | null>(null);

  watch([account, status], ([newAccount, newStatus]) => {
    if (newAccount !== undefined && newStatus === 'OPEN') {
      console.log('Authenticating WebSocket connection with JWT', newAccount.jwt);
      void sendMessage({ type: 'authenticate', token: newAccount.jwt })
        .then(() => {
          console.log('WebSocket authentication successful');
          lastAuthenticated.value = newAccount.accountId;
        })
        .catch((error: unknown) => {
          console.error('WebSocket authentication failed:', error);
        });
    }
  });

  const isAuthenticated = computed(() => {
    return (
      status.value === 'OPEN' &&
      account.value !== undefined &&
      lastAuthenticated.value === account.value.accountId
    );
  });

  const queryClient = useQueryClient();

  watch(status, (newStatus) => {
    if (newStatus === 'OPEN') {
      // Invalidate all queries that depend on WebSocket data
      void queryClient.invalidateQueries({ queryKey: ['ws'] });
    }
  });

  return { subscribe, sendMessage, isAuthenticated, lastAuthenticated };
});

export function useSubscription<T>(
  key: SubscriptionId,
  type: string,
  parser: z.ZodType<T>,
  handler: (msg: T) => void,
) {
  const websocketStore = useWebSocketStore();
  let unsubscribe: (() => void) | undefined;
  const keyValue = computed(() => JSON.stringify(key.map((k) => toValue(k))));
  watch(
    keyValue,
    (newKey) => {
      console.log('Updating subscription for key', newKey);
      if (unsubscribe !== undefined) {
        unsubscribe();
      }
      unsubscribe = websocketStore.subscribe(newKey, type, parser, handler);
    },
    { immediate: true },
  );
  onUnmounted(() => {
    if (unsubscribe !== undefined) {
      unsubscribe();
    }
  });
}
