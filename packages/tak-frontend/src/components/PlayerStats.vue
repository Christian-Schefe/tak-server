<script setup lang="ts">
import { usePlayerInfo, usePlayerStats } from '@/api/player';
import { Card } from '@tak-ui-lib/components';
import { LuFlame, LuHash, LuSwords, LuTrophy } from 'vue-icons-plus/lu';

const props = defineProps<{
  playerId: string;
}>();

const { data: playerInfo } = usePlayerInfo(() => props.playerId);
const { data: stats } = usePlayerStats(() => props.playerId);
</script>

<template>
  <div class="grid grid-cols-2 lg:grid-cols-4 justify-stretch gap-4 w-full">
    <Card class="w-full!">
      <div class="flex flex-col items-center">
        <LuHash />
        <p class="text-primary text-4xl my-4">{{ stats?.ranking?.rank ?? '...' }}</p>
        <p class="text-lg">Rank</p>
      </div>
    </Card>
    <Card class="w-full!">
      <div class="flex flex-col items-center">
        <LuTrophy />
        <p class="text-primary text-4xl my-4">
          {{ playerInfo?.participationRating?.toFixed(0) ?? '...' }}
        </p>
        <p class="text-lg">Rating</p>
      </div>
    </Card>
    <Card class="w-full!">
      <div class="flex flex-col items-center">
        <LuSwords />
        <p class="text-primary text-4xl my-4">{{ stats?.gamesPlayed ?? '...' }}</p>
        <p class="text-lg">Games</p>
      </div>
    </Card>
    <Card class="w-full!">
      <div class="flex flex-col items-center">
        <LuFlame />
        <p class="text-primary text-4xl my-4">{{ stats?.winStreak ?? '...' }}</p>
        <p class="text-lg">Win Streak</p>
      </div>
    </Card>
  </div>
</template>
