<script setup lang="ts">
import { useAccount } from '@/api/auth';
import type { ChatMessageConversation } from '@/api/chat';
import {
  useAcceptGameRequest,
  useGameStatus,
  useSetGameRequest,
  useSpectateGame,
  useUpdateGameStatus,
  type GameRequest,
  type GameRequestType,
  type GameStatus,
} from '@/api/game';
import { usePlayerInfos } from '@/api/player';
import Game, { type GameMode } from '@/components/Game.vue';
import GameOverModal from '@/components/GameOverModal.vue';
import SettingsModal from '@/components/SettingsModal.vue';
import RequestsBar from '@/components/side-panel/RequestsBar.vue';
import SidePanelAccordion from '@/components/side-panel/SidePanelAccordion.vue';
import SidePanelMobile from '@/components/side-panel/SidePanelMobile.vue';
import type { SidePanelSection } from '@/features/sidePanel';
import { usePlayGameActionSound } from '@/features/sound';
import { useWebSocketStore } from '@/features/websocket';
import {
  actionEquals,
  gameResultEquals,
  TakGame,
  type TakAction,
  type TakGameResult,
  type TakGameSettings,
} from '@/tak-core';
import { actionFromString, actionToString, gameResultFromString } from '@/tak-core/ptn';
import { produce } from 'immer';
import Button from 'primevue/button';
import Divider from 'primevue/divider';
import { computed, markRaw, ref, watch } from 'vue';
import { LuSettings } from 'vue-icons-plus/lu';
import { useRoute } from 'vue-router';

const route = useRoute('/player.[id]');

const plyIndex = ref<number | null>(null);

const webSocketStore = useWebSocketStore();
const updateGameStatus = useUpdateGameStatus(() => route.params.id);

function onAction(action: TakAction) {
  console.log('Action:', action);
  if (!gameData.value) return;
  const now = Date.now();
  updateGameStatus({
    eventType: 'gameAction',
    gameId: gameData.value.status.id,
    timeInfo: {
      white: gameData.value.game.getTimeRemaining('white', now),
      black: gameData.value.game.getTimeRemaining('black', now),
    },
    action: actionToString(action),
    plyIndex: gameData.value.game.base.actionHistory.length + 1,
  });
  void webSocketStore.sendMessage({
    type: 'gameAction',
    gameId: gameData.value.status.id,
    action: actionToString(action),
  });
}

const { data: account } = useAccount();
const { data: gameStatus } = useGameStatus(() => route.params.id);

const shouldSpectateGameId = computed(() => {
  if (!gameStatus.value || !account.value) return undefined;
  const playerIds = gameStatus.value.playerIds;
  return account.value.playerId !== playerIds.white &&
    account.value.playerId !== playerIds.black &&
    gameStatus.value.status.type === 'ongoing'
    ? route.params.id
    : undefined;
});
useSpectateGame(shouldSpectateGameId);

const gameData = computed<{
  game: TakGame;
  mode: GameMode;
  status: GameStatus;
} | null>((prevGame) => {
  if (!gameStatus.value || !account.value) {
    return null;
  }
  const gameSettings = gameStatus.value.gameSettings;
  const settings: TakGameSettings = {
    base: {
      boardSize: gameSettings.boardSize,
      halfKomi: gameSettings.halfKomi,
      reserve: {
        pieces: gameSettings.pieces,
        capstones: gameSettings.capstones,
      },
      opening: gameSettings.opening,
    },
    timeControl: gameSettings.timeSettings,
  };
  const gameResult: TakGameResult | null =
    gameStatus.value.status.type === 'ended'
      ? gameResultFromString(gameStatus.value.status.result)
      : null;

  const now = Date.now();
  const newGame =
    !!prevGame &&
    prevGame.game.base.actionHistory.every((prevAction, index) => {
      const actionRecord = gameStatus.value.actions[index];
      if (actionRecord === undefined) return true; // undone moves at end are okay
      return actionEquals(prevAction.action, actionFromString(actionRecord));
    })
      ? prevGame.game
      : markRaw(new TakGame(settings));

  const fullGame = produce(newGame, (game) => {
    const actionsToApply = gameStatus.value.actions.length - game.base.actionHistory.length;
    if (actionsToApply > 0) {
      const actionList = gameStatus.value.actions.slice(-actionsToApply);
      for (const actionRecord of actionList) {
        const action = actionFromString(actionRecord);
        if (!action) {
          console.error('Invalid action string from server:', actionRecord);
          continue;
        }
        if (!game.doAction(action, now)) {
          console.error('Failed to perform action:', action);
        }
      }
    } else if (actionsToApply < 0) {
      for (let i = 0; i < -actionsToApply; i++) {
        game.undoAction(now);
      }
    }
    game.setTimeRemaining(gameStatus.value.remainingMs, now);
    if (gameResult && !gameResultEquals(gameResult, game.base.gameResult)) {
      game.setGameOver(gameResult, now);
    }
  });
  const mode: GameMode =
    account.value.playerId === gameStatus.value.playerIds.white
      ? {
          type: 'online',
          localPlayer: 'white',
        }
      : account.value.playerId === gameStatus.value.playerIds.black
        ? {
            type: 'online',
            localPlayer: 'black',
          }
        : {
            type: 'spectator',
          };
  return {
    gameId: gameStatus.value.id,
    mode,
    game: fullGame,
    status: gameStatus.value,
  };
});

usePlayGameActionSound(() => gameData.value?.game.base);

watch(
  () => gameData.value?.game.base.actionHistory.length,
  (newLength, oldLength) => {
    if (newLength === oldLength) return;
    plyIndex.value = null;
  },
);

const settingsVisible = ref(false);

const { mutate: updateGameStatusMutation } = useSetGameRequest(() => route.params.id);

function onSetRequest(request: GameRequest) {
  updateGameStatusMutation(request);
}
const { mutate: acceptGameRequestMutation } = useAcceptGameRequest(() => route.params.id);
function onAcceptRequest(requestType: GameRequestType) {
  acceptGameRequestMutation(requestType);
}

const gameOverDialogVisible = ref(false);

watch(
  () => gameData.value?.game.base.gameResult,
  (newGameResult) => {
    if (newGameResult) {
      gameOverDialogVisible.value = true;
    }
  },
);

const accountIds = usePlayerInfos(() =>
  gameData.value?.status.playerIds
    ? [gameData.value.status.playerIds.white, gameData.value.status.playerIds.black]
    : undefined,
);

const chatConversation = computed<ChatMessageConversation | undefined>(() => {
  if (!gameData.value) return undefined;
  switch (gameData.value.mode.type) {
    case 'online': {
      const accIds = accountIds.value
        .map((info) => info.data?.accountId)
        .filter((v) => v !== undefined);
      return accIds[0] !== undefined && accIds[1] !== undefined
        ? {
            type: 'private',
            accountId1: accIds[0],
            accountId2: accIds[1],
          }
        : undefined;
    }
    case 'spectator':
      return {
        type: 'room',
        roomName: `game-${gameData.value.status.id}`,
      };
    default:
      return undefined;
  }
});

const sidePanelSections = computed<SidePanelSection[]>(() => {
  const chatSection: SidePanelSection = {
    type: 'chat',
    conversation: chatConversation.value,
  };
  if (gameData.value && gameData.value.mode.type === 'spectator') {
    return [{ type: 'analysis' }, { type: 'full_game_info' }, chatSection];
  }
  return [{ type: 'full_game_info' }, chatSection];
});
</script>

<template>
  <Game
    v-if="gameData"
    :game="gameData.game.base"
    :ply-index="plyIndex"
    :mode="gameData.mode"
    @action="onAction"
  >
    <template #desktop>
      <div class="w-full p-2 border-b border-surface flex">
        <Button
          class="w-10! h-10!"
          variant="text"
          severity="secondary"
          @click="settingsVisible = true"
        >
          <template #icon><LuSettings></LuSettings></template>
        </Button>
        <template
          v-if="gameData.mode.type === 'online' && gameData.status.status.type === 'ongoing'"
        >
          <Divider layout="vertical" class="mx-2!"></Divider>
          <RequestsBar
            :white-requests="gameData.status.status.whiteRequests"
            :black-requests="gameData.status.status.blackRequests"
            :player="gameData.mode.localPlayer"
            @set-request="onSetRequest"
            @accept-request="onAcceptRequest"
          />
        </template>
        <template v-if="gameData.status.status.type !== 'ongoing'">
          <Divider layout="vertical" class="mx-2!"></Divider>
          <Button label="View Result" severity="secondary" @click="gameOverDialogVisible = true" />
        </template>
        <SettingsModal v-model="settingsVisible"></SettingsModal>
        <GameOverModal
          v-model="gameOverDialogVisible"
          :result="
            gameData.status.status.type === 'ended'
              ? gameResultFromString(gameData.status.status.result)
              : { type: 'aborted' }
          "
          :match-id="gameData.status.matchId"
        />
      </div>
      <SidePanelAccordion
        v-model:ply-index="plyIndex"
        :game="gameData.game.base"
        :full-game="gameData.game"
        :player-ids="gameData.status.playerIds"
        :sections="sidePanelSections"
      ></SidePanelAccordion>
    </template>
    <template #mobile>
      <SidePanelMobile
        v-model:ply-index="plyIndex"
        :game="gameData.game.base"
        :full-game="gameData.game"
        :player-ids="gameData.status.playerIds"
        :sections="sidePanelSections"
      />
    </template>
  </Game>
</template>
