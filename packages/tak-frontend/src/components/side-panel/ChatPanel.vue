<script setup lang="ts">
import { useChatHistory, useSendChatMessage, type ChatMessageConversation } from '@/api/chat';
import { areTimestampsDifferentMinutes } from '@/utils/time';
import { Form, type FormFieldState, type FormSubmitEvent } from '@primevue/forms';
import { isToday } from 'date-fns';
import Button from 'primevue/button';
import ScrollPanel from 'primevue/scrollpanel';
import Textarea from 'primevue/textarea';
import { computed, useTemplateRef, watch } from 'vue';
import { LuSend } from 'vue-icons-plus/lu';
import VueMarkdown from 'vue-markdown-render';
import PlayerLabel from '../PlayerLabel.vue';

const props = defineProps<{
  conversation: ChatMessageConversation;
}>();

const {
  data: chatHistory,
  fetchNextPage,
  hasNextPage,
  isFetchingNextPage,
} = useChatHistory(props.conversation);

const messages = computed(() => {
  if (!chatHistory.value) {
    return [];
  }
  const messagesSorted = chatHistory.value.pages
    .flatMap((page) => page.messages)
    .sort((a, b) => a.timestamp - b.timestamp);

  return messagesSorted.map((message, i) => {
    const prev = i > 0 ? messagesSorted[i - 1] : null;
    const showTimestamp = !prev || areTimestampsDifferentMinutes(prev.timestamp, message.timestamp);
    const showDateAndTimestamp = showTimestamp && !isToday(message.timestamp);

    let formattedTimestamp: string | null = null;
    if (showDateAndTimestamp) {
      formattedTimestamp = new Date(message.timestamp).toLocaleDateString([], {
        month: 'short',
        day: 'numeric',
        year: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
      });
    } else if (showTimestamp) {
      formattedTimestamp = new Date(message.timestamp).toLocaleTimeString([], {
        hour: '2-digit',
        minute: '2-digit',
      });
    }
    return {
      ...message,
      formattedTimestamp,
    };
  });
});

const sendMessage = useSendChatMessage(props.conversation);

function maybeSendMessage(message: unknown) {
  if (typeof message === 'string' && message.trim() !== '') {
    void sendMessage(message);
    return true;
  }
  return false;
}

function onSendMessage(event: FormSubmitEvent) {
  const formData = event.values as { chatMessage: unknown };
  maybeSendMessage(formData.chatMessage);
  event.reset();
}

watch(messages, (newMessages, oldMessages) => {
  if (newMessages.length > 0) {
    const latestMessage = newMessages[newMessages.length - 1];
    if (!latestMessage) {
      return;
    }
    const isNew = oldMessages[oldMessages.length - 1]?.messageId !== latestMessage.messageId;
    if (!isNew) {
      return;
    }
    scrollToMessage(latestMessage.messageId);
  }
});

const messageContainer = useTemplateRef('messageContainer');

function scrollToMessage(messageId: number) {
  setTimeout(() => {
    const element = messageContainer.value?.querySelector(`.message-${messageId.toString()}`) as
      | Element
      | null
      | undefined;
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }
  }, 10);
}
function textareaKeydown(
  event: KeyboardEvent,
  formValues: Record<string, FormFieldState> & {
    reset: () => void;
  },
) {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    maybeSendMessage(formValues.chatMessage?.value);
    formValues.reset();
  }
}
</script>
<template>
  <div class="flex flex-col h-full gap-2">
    <div class="h-0 grow flex flex-col">
      <ScrollPanel class="h-0 grow">
        <div ref="messageContainer" class="flex flex-col items-center">
          <Button
            v-if="hasNextPage"
            label="Load More"
            variant="text"
            :disabled="isFetchingNextPage"
            @click="void fetchNextPage()"
          ></Button>
          <div
            v-for="message in messages"
            :key="message.messageId"
            :class="`message-${message.messageId}`"
            class="w-full flex flex-col mt-4"
          >
            <div class="flex items-center gap-4">
              <PlayerLabel
                :pid="message.sender"
                type="account"
                :show-flag="false"
                :show-rating="false"
              />
              <p v-if="message.formattedTimestamp" class="text-sm text-muted-color">
                {{ message.formattedTimestamp }}
              </p>
            </div>
            <div class="pl-10 w-full text-muted-color markdown-body">
              <VueMarkdown :source="message.message" />
            </div>
          </div>
        </div>
      </ScrollPanel>
    </div>
    <Form v-slot="$form" class="h-10 flex gap-2 w-full items-end" @submit="onSendMessage">
      <Textarea
        name="chatMessage"
        rows="1"
        auto-resize
        cols="30"
        class="resize-none w-0 grow z-1"
        @keydown="(event) => textareaKeydown(event, $form)"
      ></Textarea>
      <Button class="h-10! w-10! p-0!" type="submit">
        <template #icon>
          <LuSend />
        </template>
      </Button>
    </Form>
  </div>
</template>
<style lang="css">
.markdown-body p {
  line-break: anywhere;
}

.markdown-body h1 {
  font-size: 2rem;
  font-weight: bold;
}

.markdown-body h2 {
  font-size: 1.5rem;
  font-weight: bold;
}

.markdown-body h3 {
  font-size: 1.25rem;
  font-weight: bold;
}

.markdown-body ul,
.markdown-body ol {
  padding-left: 1.5rem;
}

.markdown-body ul li {
  list-style-type: disc;
}

.markdown-body ol li {
  list-style-type: decimal;
}

.markdown-body a {
  color: var(--p-primary-color);
  text-decoration: none;
}

.markdown-body a:hover {
  text-decoration: underline;
}

.markdown-body blockquote {
  border-left: 2px solid var(--p-primary-color);
  padding-left: 0.5rem;
  margin: 1rem 0;
}

.markdown-body table {
  border-collapse: collapse;
  width: 100%;
  margin: 1rem 0;
}

.markdown-body table th,
.markdown-body table td {
  border: 1px solid var(--color-surface-500);
  padding: 0.5rem;
  text-align: left;
}

.markdown-body table th {
  font-weight: bold;
}

.markdown-body code {
  font-family: 'Courier New', Courier, monospace;
}
</style>
