<script setup lang="ts">
import { usePlayerStats } from '@/api/player';
import { ArcElement, Chart, type ChartData, type ChartOptions } from 'chart.js';
import { computed } from 'vue';
import { Doughnut } from 'vue-chartjs';

Chart.register(ArcElement);

const props = defineProps<{
  playerId: string;
}>();

const { data: stats } = usePlayerStats(() => props.playerId);

const wdl = computed(() => {
  if (!stats.value) {
    return undefined;
  }
  const sum = stats.value.gamesWon + stats.value.gamesDrawn + stats.value.gamesLost;
  const safeSum = sum === 0 ? 1 : sum;
  const winPercent = Math.round((stats.value.gamesWon * 100) / safeSum);
  const lossPercent = Math.round((stats.value.gamesLost * 100) / safeSum);
  const drawPercent = sum === 0 ? 0 : 100 - winPercent - lossPercent;

  const documentStyle = getComputedStyle(document.documentElement);

  const winColor = documentStyle.getPropertyValue('--p-color-primarycontainer');
  const lossColor = documentStyle.getPropertyValue('--p-color-errorcontainer');
  const drawColor = documentStyle.getPropertyValue('--p-color-surfacecontainerhighest');
  const textColor = documentStyle.getPropertyValue('--p-color-onsurface');

  return {
    data: {
      labels: [
        `${stats.value.gamesWon.toString()} Win${stats.value.gamesWon !== 1 ? 's' : ''}`,
        `${stats.value.gamesDrawn.toString()} Draw${stats.value.gamesDrawn !== 1 ? 's' : ''}`,
        `${stats.value.gamesLost.toString()} Loss${stats.value.gamesLost !== 1 ? 'es' : ''}`,
      ],
      datasets: [
        {
          data: [winPercent, drawPercent, lossPercent],
          borderColor: textColor,
          backgroundColor: [winColor, drawColor, lossColor],
          borderWidth: 1,
        },
      ],
    } satisfies ChartData<'doughnut'>,
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: {
          position: 'bottom',
          labels: {
            color: textColor,
            font: {
              size: 14,
            },
          },
        },
      },
    } satisfies ChartOptions<'doughnut'>,
  };
});
</script>
<template>
  <div class="w-full h-96">
    <Doughnut v-if="wdl" :data="wdl.data" :options="wdl.options"></Doughnut>
  </div>
</template>
