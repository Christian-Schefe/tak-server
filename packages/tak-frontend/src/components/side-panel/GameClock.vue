<script setup lang="ts">
import PlayerLabel from '@/components/PlayerLabel.vue';
import type { TakGame, TakPlayer } from '@/tak-core';
import { clockFormat } from '@/utils/time';
import { useInterval } from '@vueuse/core';
import { computed } from 'vue';

const props = defineProps<{
  game: TakGame;
  player: TakPlayer;
  playerId: string;
}>();

const counter = useInterval(100);

const clockInfo = computed(() => {
  const remainingMs = props.game.getTimeRemaining(props.player, Date.now());
  const isActive = props.player === props.game.base.currentPlayer && props.game.clock.isTicking;
  return { remainingMs: clockFormat(remainingMs), isActive, counter: counter.value };
});
</script>
<template>
  <div class="flex items-center gap-2">
    <PlayerLabel :pid="playerId" type="player" />
    <div class="grow"></div>
    <div
      :class="`py-2 px-4 text-lg text-center rounded-md font-mono min-w-24 border ${clockInfo.isActive ? 'bg-surface-700 dark:bg-surface-200' : 'border-surface'} ${clockInfo.isActive ? 'text-primary-contrast' : 'text-muted-color'} transition-colors`"
    >
      <p>{{ clockInfo.remainingMs }}</p>
    </div>
  </div>
</template>
