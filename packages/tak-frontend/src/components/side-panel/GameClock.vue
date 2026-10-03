<script setup lang="ts">
import PlayerLabel from '@/components/PlayerLabel.vue';
import type { TakGame, TakPlayer } from '@/tak-core';
import { clockFormat } from '@/utils/time';
import { Tag } from '@tak-ui-lib/components';
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
    <Tag
      :class="`justify-center font-mono min-w-24`"
      :style="{
        opacity: clockInfo.isActive ? 1 : 0.5,
        transition: 'opacity 200ms ease-in-out',
      }"
      :label="clockInfo.remainingMs"
    />
  </div>
</template>
