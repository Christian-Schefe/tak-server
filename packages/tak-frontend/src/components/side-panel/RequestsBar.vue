<script setup lang="ts">
import type { GameRequest, GameRequests, GameRequestType } from '@/api/game';
import type { TakPlayer } from '@/tak-core';
import { Button, ButtonGroup } from '@tak-ui-lib/components';
import { computed } from 'vue';
import { LuHeartHandshake, LuUndo2, LuCheck, LuClock } from 'vue-icons-plus/lu';

const props = defineProps<{
  whiteRequests: GameRequests;
  blackRequests: GameRequests;
  player: TakPlayer;
}>();

const emit = defineEmits<{
  (e: 'setRequest', request: GameRequest): void;
  (e: 'acceptRequest', requestType: GameRequestType): void;
}>();

const requests = computed(() => ({
  thisPlayer: props.player === 'white' ? props.whiteRequests : props.blackRequests,
  otherPlayer: props.player === 'white' ? props.blackRequests : props.whiteRequests,
}));

function onClickDraw() {
  emit('setRequest', {
    type: 'draw',
    offer: !requests.value.thisPlayer.drawOffered,
  });
}

function onClickUndo() {
  emit('setRequest', {
    type: 'undo',
    request: !requests.value.thisPlayer.undoRequested,
  });
}

function onClickMoreTime() {
  emit('setRequest', {
    type: 'moreTime',
    amountMs: requests.value.thisPlayer.moreTimeOffered === null ? 30000 : null,
  });
}
</script>
<template>
  <div class="grow flex gap-2">
    <ButtonGroup>
      <Button
        icon-only
        size="small"
        :variant="requests.otherPlayer.drawOffered ? 'filled' : 'text'"
        :severity="requests.thisPlayer.drawOffered ? 'danger' : 'secondary'"
        @click="onClickDraw"
      >
        <LuHeartHandshake></LuHeartHandshake>
      </Button>
      <Button
        v-if="requests.otherPlayer.drawOffered"
        variant="filled"
        size="small"
        icon-only
        @click="emit('acceptRequest', 'draw')"
      >
        <LuCheck></LuCheck>
      </Button>
    </ButtonGroup>
    <ButtonGroup>
      <Button
        icon-only
        size="small"
        :variant="requests.otherPlayer.undoRequested ? 'filled' : 'text'"
        :severity="requests.thisPlayer.undoRequested ? 'danger' : 'secondary'"
        @click="onClickUndo"
      >
        <LuUndo2></LuUndo2>
      </Button>
      <Button
        v-if="requests.otherPlayer.undoRequested"
        icon-only
        size="small"
        variant="filled"
        @click="emit('acceptRequest', 'undo')"
      >
        <LuCheck></LuCheck>
      </Button> </ButtonGroup
    ><ButtonGroup>
      <Button
        icon-only
        size="small"
        :variant="requests.otherPlayer.moreTimeOffered ? 'filled' : 'text'"
        :severity="requests.thisPlayer.moreTimeOffered ? 'danger' : 'secondary'"
        @click="onClickMoreTime"
      >
        <LuClock></LuClock>
      </Button>
      <Button
        v-if="requests.otherPlayer.moreTimeOffered"
        size="small"
        variant="filled"
        icon-only
        @click="emit('acceptRequest', 'moreTime')"
      >
        <LuCheck></LuCheck>
      </Button>
    </ButtonGroup>
  </div>
</template>
