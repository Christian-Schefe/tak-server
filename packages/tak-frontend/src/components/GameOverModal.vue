<script setup lang="ts">
import { useMatchSetPlayerReady } from '@/api/match';
import type { TakGameResult } from '@/tak-core';
import Button from 'primevue/button';
import Dialog from 'primevue/dialog';
import { computed } from 'vue';
import { useRouter } from 'vue-router';

const visible = defineModel<boolean>({ required: true });

const props = defineProps<{
  result: TakGameResult;
  matchId: string | null;
}>();

const resultText = computed(() => {
  if (props.result.type === 'win') {
    let reasonText;
    switch (props.result.reason) {
      case 'resignation':
        reasonText = 'resignation';
        break;
      case 'timeout':
        reasonText = 'timeout';
        break;
      case 'flats':
        reasonText = props.result.counts
          ? `flat count (${props.result.counts.white.toString()} to ${props.result.counts.black.toString()})`
          : 'flat count';
        break;
      case 'road':
        reasonText = 'forming a road';
        break;
      case 'default':
        reasonText = 'default';
        break;
    }
    return `${props.result.winner === 'white' ? 'White' : 'Black'} wins by ${reasonText}!`;
  } else if (props.result.type === 'draw') {
    return `The game is a draw.`;
  } else {
    return 'The game was aborted.';
  }
});

const router = useRouter();

function goToMatch() {
  if (props.matchId === null) return;
  void router.push(`/match/${props.matchId}`);
}

const { mutate: setPlayerReady } = useMatchSetPlayerReady();

function rematch() {
  if (props.matchId === null) return;
  setPlayerReady({ id: props.matchId, ready: true });
  void router.push(`/match/${props.matchId}`);
}
</script>
<template>
  <Dialog
    v-model:visible="visible"
    dismissable-mask
    header="Game Over"
    :draggable="false"
    modal
    :style="{ width: '90vw', maxWidth: '600px' }"
  >
    <div class="w-full h-[30vh] flex flex-col">
      <div class="w-full grow">
        <p>{{ resultText }}</p>
      </div>
      <div class="w-full grid grid-cols-2 gap-4">
        <Button severity="secondary" label="Go to Match" @click="goToMatch" />
        <Button label="Rematch" @click="rematch" />
      </div>
    </div>
  </Dialog>
</template>
