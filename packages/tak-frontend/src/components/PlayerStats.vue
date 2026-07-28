<script setup lang="ts">
import { usePlayerInfo, usePlayerStats } from '@/api/player';
import Card from 'primevue/card';
import { LuHash, LuTrophy, LuSwords, LuFlame } from 'vue-icons-plus/lu';
import MeterGroup, { type MeterItem } from 'primevue/metergroup';
import { computed } from 'vue';

const props = defineProps<{
  playerId: string;
}>();

const { data: playerInfo } = usePlayerInfo(() => props.playerId);
const { data: stats } = usePlayerStats(() => props.playerId);

const wdlItems = computed<MeterItem[]>(() => {
  if (!stats.value) {
    return [];
  }
  const sum = stats.value.gamesWon + stats.value.gamesDrawn + stats.value.gamesLost;
  const safeSum = sum === 0 ? 1 : sum;
  const winPercent = Math.round((stats.value.gamesWon * 100) / safeSum);
  const lossPercent = Math.round((stats.value.gamesLost * 100) / safeSum);
  const drawPercent = sum === 0 ? 0 : 100 - winPercent - lossPercent;
  return [
    {
      label: `${stats.value.gamesWon.toString()} Win${stats.value.gamesWon !== 1 ? 's' : ''}`,
      value: winPercent,
      color: 'var(--p-green-500)',
    },
    {
      label: `${stats.value.gamesDrawn.toString()} Draw${stats.value.gamesDrawn !== 1 ? 's' : ''}`,
      value: drawPercent,
      color: 'var(--p-neutral-400)',
    },
    {
      label: `${stats.value.gamesLost.toString()} Loss${stats.value.gamesLost !== 1 ? 'es' : ''}`,
      value: lossPercent,
      color: 'var(--p-red-500)',
    },
  ];
});
</script>

<template>
  <div class="grid grid-cols-2 lg:grid-cols-4 justify-stretch gap-4 w-full">
    <Card class="w-full!">
      <template #content>
        <div class="flex flex-col items-center">
          <LuHash />
          <p class="text-primary text-4xl my-4">{{ stats?.ranking?.rank ?? '...' }}</p>
          <p class="text-lg">Rank</p>
        </div>
      </template>
    </Card>
    <Card class="w-full!">
      <template #content>
        <div class="flex flex-col items-center">
          <LuTrophy />
          <p class="text-primary text-4xl my-4">
            {{ playerInfo?.participationRating?.toFixed(0) ?? '...' }}
          </p>
          <p class="text-lg">Rating</p>
        </div>
      </template>
    </Card>
    <Card class="w-full!">
      <template #content>
        <div class="flex flex-col items-center">
          <LuSwords />
          <p class="text-primary text-4xl my-4">{{ stats?.gamesPlayed ?? '...' }}</p>
          <p class="text-lg">Games</p>
        </div>
      </template>
    </Card>
    <Card class="w-full!">
      <template #content>
        <div class="flex flex-col items-center">
          <LuFlame />
          <p class="text-primary text-4xl my-4">{{ stats?.winStreak ?? '...' }}</p>
          <p class="text-lg">Win Streak</p>
        </div>
      </template>
    </Card>
  </div>
  <MeterGroup
    :value="wdlItems"
    :dt="{
      meters: {
        size: '1rem',
      },
    }"
  ></MeterGroup>
</template>
