<script setup lang="ts">
import type { GameMode } from '@/components/Game.vue';
import { useSettingsStore } from '@/features/settings';
import type { TakAction, TakBaseGame } from '@/tak-core';
import { actionFromString, gameToPTN } from '@/tak-core/ptn';
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue';
import { z } from 'zod';

const NinjaMessageSchema = z.object({
  action: z.string(),
  value: z.any(),
});

const params =
  '&moveNumber=false&unplayedPieces=true&disableStoneCycling=true&showBoardPrefsBtn=false&disableNavigation=true&disablePTN=true&disableText=true&flatCounts=false&turnIndicator=false&showHeader=false&showEval=false&showRoads=false&stackCounts=false&notifyGame=false';
const ninjaSrc = `https://ptn.ninja/${params}`;

const emit = defineEmits<{
  (e: 'action', action: TakAction): void;
}>();

const props = defineProps<{
  game: TakBaseGame;
  plyIndex: number | null;
  mode: GameMode;
}>();

const hasLoaded = ref(false);

const iframe = useTemplateRef('iframe');

function sendMessageToIframe(message: unknown) {
  iframe.value?.contentWindow?.postMessage(message, 'https://ptn.ninja');
}

const shouldBoardBeDisabled = computed(() => {
  return (
    props.mode.type === 'spectator' ||
    props.plyIndex !== null ||
    !props.game.isOngoing() ||
    (props.mode.type === 'online' && props.mode.localPlayer !== props.game.currentPlayer)
  );
});

watch([hasLoaded, shouldBoardBeDisabled], ([newHasLoaded, newShouldBoardBeDisabled]) => {
  if (!newHasLoaded) return;
  sendMessageToIframe({
    action: 'SET_UI',
    value: {
      disableBoard: newShouldBoardBeDisabled,
    },
  });
});

const settingsStore = useSettingsStore();

watch(
  [hasLoaded, () => settingsStore.settings.boardTypeSettings.ninja],
  ([newHasLoaded, newSettings]) => {
    if (!newHasLoaded) return;
    sendMessageToIframe({
      action: 'SET_UI',
      value: {
        theme: newSettings.colorTheme,
        axisLabels: newSettings.axisLabels !== 'none',
        axisLabelsSmall: newSettings.axisLabels === 'small',
        highlightSquares: newSettings.highlightSquares,
        animateBoard: newSettings.animateBoard,
        board3D: newSettings.board3d,
        orthographic: newSettings.orthographic,
        perspective: newSettings.perspective,
      },
    });
  },
  { deep: true },
);

watch(
  [hasLoaded, () => props.game, () => props.plyIndex],
  ([newHasLoaded, newGame, newPlyIndex]) => {
    if (!newHasLoaded) return;

    const ptn = gameToPTN(newGame.settings, newGame.actionHistory, newGame.gameResult);

    sendMessageToIframe({
      action: 'SET_CURRENT_PTN',
      value: ptn,
    });
    if (newPlyIndex === null) {
      sendMessageToIframe({
        action: 'LAST',
        value: null,
      });
    } else {
      if (newPlyIndex === 0) {
        sendMessageToIframe({
          action: 'FIRST',
          value: null,
        });
      } else {
        sendMessageToIframe({
          action: 'GO_TO_PLY',
          value: {
            plyID: newPlyIndex - 1,
            isDone: true,
          },
        });
      }
    }
  },
);

function onMessage(event: MessageEvent) {
  if (event.origin !== 'https://ptn.ninja') return;
  const parsed = NinjaMessageSchema.safeParse(event.data);
  if (!parsed.success) return;
  const message = parsed.data;
  if (message.action === 'GAME_STATE' && !hasLoaded.value) {
    hasLoaded.value = true;
  } else if (hasLoaded.value && message.action === 'INSERT_PLY') {
    const action = actionFromString(message.value as string);
    if (!action) {
      console.warn('Received invalid action from Board Ninja iframe:', message.value);
      return;
    }
    emit('action', action);
  }
}
onMounted(() => {
  window.addEventListener('message', onMessage);
});

onBeforeUnmount(() => {
  window.removeEventListener('message', onMessage);
});
</script>

<template>
  <iframe
    ref="iframe"
    :src="ninjaSrc"
    class="w-full h-full min-h-[60vh] border-none rounded-md overflow-hidden"
  ></iframe>
</template>
