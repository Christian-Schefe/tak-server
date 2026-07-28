<script setup lang="ts">
import type { GameRequest, GameRequests, GameRequestType } from '@/api/game';
import type { TakPlayer } from '@/tak-core';
import Button from 'primevue/button';
import ButtonGroup from 'primevue/buttongroup';
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
        class="w-10! h-10!"
        :severity="requests.thisPlayer.drawOffered ? 'danger' : 'secondary'"
        @click="onClickDraw"
      >
        <template #icon><LuHeartHandshake></LuHeartHandshake></template>
      </Button>
      <Button
        v-if="requests.otherPlayer.drawOffered"
        class="w-10! h-10!"
        @click="emit('acceptRequest', 'draw')"
      >
        <template #icon><LuCheck></LuCheck></template>
      </Button>
    </ButtonGroup>
    <ButtonGroup>
      <Button
        class="w-10! h-10!"
        :severity="requests.thisPlayer.undoRequested ? 'danger' : 'secondary'"
        @click="onClickUndo"
      >
        <template #icon><LuUndo2></LuUndo2></template>
      </Button>
      <Button
        v-if="requests.otherPlayer.undoRequested"
        class="w-10! h-10!"
        @click="emit('acceptRequest', 'undo')"
      >
        <template #icon><LuCheck></LuCheck></template>
      </Button> </ButtonGroup
    ><ButtonGroup>
      <Button
        class="w-10! h-10!"
        :severity="requests.thisPlayer.moreTimeOffered ? 'danger' : 'secondary'"
        @click="onClickMoreTime"
      >
        <template #icon><LuClock></LuClock></template>
      </Button>
      <Button
        v-if="requests.otherPlayer.moreTimeOffered"
        class="w-10! h-10!"
        @click="emit('acceptRequest', 'moreTime')"
      >
        <template #icon><LuCheck></LuCheck></template>
      </Button>
    </ButtonGroup>
  </div>
</template>
