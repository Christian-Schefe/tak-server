import { useSubscription, useWebSocketStore } from '@/features/websocket';
import { useFetch } from '@/utils/fetch';
import { useInfiniteQuery, useQueryClient, type InfiniteData } from '@tanstack/vue-query';
import { computed, onUnmounted, ref, toValue, watch, type MaybeRefOrGetter } from 'vue';
import { z } from 'zod';

export type ChatMessage = z.infer<typeof historyChatMessage>;

export const chatMessageConversation = z.union([
  z.object({ type: z.literal('global') }),
  z.object({ type: z.literal('room'), roomName: z.string() }),
  z.object({ type: z.literal('private'), accountId1: z.string(), accountId2: z.string() }),
]);

export const historyChatMessage = z.object({
  messageId: z.number(),
  sender: z.string(),
  message: z.string(),
  timestamp: z.number(),
});

export const wsChatMessage = z.object({
  conversation: chatMessageConversation,
  message: historyChatMessage,
});

const chatHistoryPage = z.object({
  messages: z.array(historyChatMessage),
  nextCursor: z.number().nullable(),
});

export type ChatHistoryPage = z.infer<typeof chatHistoryPage>;
export type ChatMessageConversation = z.infer<typeof chatMessageConversation>;

function getConversationId(conversation: ChatMessageConversation): string {
  if (conversation.type === 'global') {
    return 'global';
  } else if (conversation.type === 'room') {
    return `room:${conversation.roomName}`;
  } else {
    const ids = [conversation.accountId1, conversation.accountId2].sort();
    return `private:${ids.join(':')}`;
  }
}

const joinedChatRooms = new Map<string, number>();
function joinChatRoom(roomName: string) {
  const count = joinedChatRooms.get(roomName) ?? 0;
  joinedChatRooms.set(roomName, count + 1);
  return count === 0;
}

function leaveChatRoom(roomName: string) {
  const count = joinedChatRooms.get(roomName) ?? 0;
  if (count <= 1) {
    joinedChatRooms.delete(roomName);
    return true; // last subscriber left
  }
  joinedChatRooms.set(roomName, count - 1);
  return false;
}

function useJoinChatRoom(roomName: MaybeRefOrGetter<string | undefined>) {
  const websocketStore = useWebSocketStore();
  const currentSubscription = ref<string | null>(null);

  watch(
    [() => websocketStore.isAuthenticated, () => toValue(roomName)],
    ([newIsAuthenticated, newRoomName]) => {
      if (!newIsAuthenticated) {
        joinedChatRooms.clear();
        currentSubscription.value = null;
        return;
      }
      if (currentSubscription.value !== null && newRoomName !== currentSubscription.value) {
        const shouldLeaveRoom = leaveChatRoom(currentSubscription.value);
        if (shouldLeaveRoom) {
          console.log('Unsubscribing from join room with name', currentSubscription.value);
          joinedChatRooms.delete(currentSubscription.value);
          void websocketStore.sendMessage({
            type: 'joinChatRoom',
            roomName: currentSubscription.value,
            join: false,
          });
        }
      }
      if (
        newRoomName !== undefined &&
        (currentSubscription.value === null || newRoomName !== currentSubscription.value)
      ) {
        const shouldJoinRoom = joinChatRoom(newRoomName);
        if (shouldJoinRoom) {
          console.log('Subscribing to join room with name', newRoomName);
          void websocketStore.sendMessage({
            type: 'joinChatRoom',
            roomName: newRoomName,
            join: true,
          });
        }
        currentSubscription.value = newRoomName;
      } else {
        currentSubscription.value = null;
      }
    },
    { immediate: true },
  );
  onUnmounted(() => {
    if (currentSubscription.value === null) return;
    const shouldLeaveRoom = leaveChatRoom(currentSubscription.value);
    if (shouldLeaveRoom) {
      console.log('Unsubscribing from join room with name', currentSubscription.value);
      void websocketStore.sendMessage({
        type: 'joinChatRoom',
        roomName: currentSubscription.value,
        join: false,
      });
    }
  });
}

export function useChatHistory(conversation: MaybeRefOrGetter<ChatMessageConversation>) {
  const conversationId = computed(() => getConversationId(toValue(conversation)));
  const { fetchTyped } = useFetch();

  const roomName = computed(() => {
    const conv = toValue(conversation);
    if (conv.type === 'room') {
      return conv.roomName;
    } else {
      return undefined;
    }
  });
  useJoinChatRoom(roomName);

  const queryClient = useQueryClient();
  useSubscription(['chatMessage'], 'chatMessage', wsChatMessage, (message) => {
    const conversationId = getConversationId(message.conversation);
    console.log('Received chat message for conversation', conversationId, message);
    queryClient.setQueryData<InfiniteData<ChatHistoryPage>>(
      ['ws', 'chatHistory', conversationId],
      (oldData) => {
        if (!oldData) {
          return oldData;
        }
        const firstPage = oldData.pages[0];
        if (firstPage) {
          return {
            ...oldData,
            pages: [
              ...oldData.pages.slice(1),
              {
                ...firstPage,
                messages: [message.message, ...firstPage.messages],
              },
            ],
          };
        } else {
          return {
            ...oldData,
            pages: [
              ...oldData.pages,
              {
                messages: [message.message],
                nextCursor: null,
              },
            ],
          };
        }
      },
    );
  });

  return useInfiniteQuery<ChatHistoryPage>({
    queryKey: ['ws', 'chatHistory', conversationId],
    queryFn: async ({ pageParam }) => {
      const cursor = pageParam as number | null;
      return await fetchTyped(
        chatHistoryPage,
        `/api/chat/${conversationId.value}?limit=20${cursor !== null ? `&cursor=${cursor.toString()}` : ''}`,
      );
    },
    initialPageParam: null,
    getNextPageParam: (lastPage) => lastPage.nextCursor,
  });
}

export function useSendChatMessage(conversation: MaybeRefOrGetter<ChatMessageConversation>) {
  const webSocketStore = useWebSocketStore();
  return async function sendMessage(message: string) {
    await webSocketStore.sendMessage({ type: 'chatMessage', conversation, message });
  };
}
