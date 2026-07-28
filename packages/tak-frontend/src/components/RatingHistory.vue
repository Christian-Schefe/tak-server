<script setup lang="ts">
import { useRatingHistory } from '@/api/player';
import { endOfDay, startOfDay, subDays, subYears } from 'date-fns';
import Chart from 'primevue/chart';
import Select from 'primevue/select';
import { computed, ref } from 'vue';
import type { ChartConfiguration } from 'chart.js';

const props = defineProps<{
  playerId: string;
}>();

const ratingRangeOptions: { label: string; value: RatingRangeOptionKey }[] = [
  { label: 'Last 7 days', value: 'last-7-days' },
  { label: 'Last 30 days', value: 'last-30-days' },
  { label: 'Last 90 days', value: 'last-90-days' },
  { label: 'Last year', value: 'last-year' },
  { label: 'All time', value: 'all-time' },
];

const ratingRangeOptionsMap = computed(() => {
  const now = new Date();
  const startOfToday = startOfDay(now);
  const endOfToday = endOfDay(now);
  const options: Record<RatingRangeOptionKey, { from: Date | null; to: Date }> = {
    'last-7-days': { from: subDays(startOfToday, 6), to: endOfToday },
    'last-30-days': { from: subDays(startOfToday, 29), to: endOfToday },
    'last-90-days': { from: subDays(startOfToday, 89), to: endOfToday },
    'last-year': { from: subYears(startOfToday, 1), to: endOfToday },
    'all-time': { from: null, to: endOfToday },
  };
  return options;
});

const ratingRange = ref<RatingRangeOptionKey>('last-30-days');
const selectedRange = computed(() => ratingRangeOptionsMap.value[ratingRange.value]);

const { data: ratingHistoryData } = useRatingHistory(
  () => props.playerId,
  () => selectedRange.value.from?.getTime(),
  () => selectedRange.value.to.getTime(),
);

type RatingRangeOptionKey =
  | 'last-7-days'
  | 'last-30-days'
  | 'last-90-days'
  | 'last-year'
  | 'all-time';

const ratingHistory = computed(() => {
  const data = ratingHistoryData.value;
  const entries = data ? [...data.entries] : [];
  if (data?.firstEntryBeforeRange) {
    entries.push(data.firstEntryBeforeRange);
  }
  const firstEntry = entries[0];
  if (firstEntry) {
    const lastRating = firstEntry.rating;
    entries.unshift({
      timestamp: Date.now(),
      rating: lastRating,
    });
  }
  const dataEntries = entries
    .map((entry) => ({
      x: new Date(entry.timestamp).getTime(),
      y: Math.round(entry.rating),
    }))
    .reverse();
  const documentStyle = getComputedStyle(document.documentElement);
  const primaryColor = documentStyle.getPropertyValue('--p-primary-color');

  const textColorSecondary = documentStyle.getPropertyValue('--p-text-muted-color');
  const surfaceBorder = documentStyle.getPropertyValue('--p-content-border-color');

  const yMin = dataEntries.length > 0 ? Math.min(...dataEntries.map((d) => d.y)) : undefined;
  const yMax = dataEntries.length > 0 ? Math.max(...dataEntries.map((d) => d.y)) : undefined;

  const nearestSmallerHundred = (num: number) => Math.floor(num / 100) * 100;
  const nearestLargerHundred = (num: number) => Math.ceil(num / 100) * 100;

  const config: ChartConfiguration = {
    type: 'line',
    options: {
      responsive: true,
      maintainAspectRatio: false,
      scales: {
        x: {
          type: 'time',
          time: {
            minUnit: 'day',
          },
          min: selectedRange.value.from ? selectedRange.value.from.getTime() : undefined,
          max: selectedRange.value.to.getTime(),
          ticks: {
            color: textColorSecondary,
          },
          grid: {
            color: surfaceBorder,
          },
        },
        y: {
          beginAtZero: false,
          min: yMin !== undefined ? nearestSmallerHundred(yMin - 10) : undefined,
          max: yMax !== undefined ? nearestLargerHundred(yMax + 10) : undefined,
          ticks: {
            color: textColorSecondary,
          },
          grid: {
            color: surfaceBorder,
          },
        },
      },
      plugins: {
        legend: {
          display: false,
        },
      },
      interaction: {
        mode: 'nearest',
        intersect: false,
      },
    },
    data: {
      datasets: [
        {
          borderColor: primaryColor,
          data: dataEntries,
          stepped: true,
        },
      ],
    },
  };
  return config;
});
</script>
<template>
  <div class="flex flex-col gap-4">
    <Select
      v-model="ratingRange"
      :options="ratingRangeOptions"
      option-label="label"
      option-value="value"
    />
    <Chart
      type="line"
      :data="ratingHistory.data"
      :options="ratingHistory.options"
      class="h-64"
    ></Chart>
  </div>
</template>
