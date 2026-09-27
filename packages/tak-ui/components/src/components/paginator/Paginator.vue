<script setup lang="ts">
import { computed } from 'vue';
import { Button } from '../button';

const currentPage = defineModel<number>({ default: 1 });

const props = withDefaults(
  defineProps<{
    totalItems: number;
    itemsPerPage: number;
    maxPageCountSingle?: number;
  }>(),
  {
    maxPageCountSingle: 2,
  },
);

const shownPages = computed(() => {
  if (props.itemsPerPage <= 0) return [];
  const totalPages = Math.ceil(props.totalItems / props.itemsPerPage);
  const pages: number[] = [];
  const leftOverflowAmount = Math.max(0, props.maxPageCountSingle - currentPage.value + 1);
  const rightOverflowAmount = Math.max(
    0,
    props.maxPageCountSingle - (totalPages - currentPage.value),
  );
  const minPage = Math.max(1, currentPage.value - props.maxPageCountSingle - rightOverflowAmount);
  const maxPage = Math.min(
    totalPages,
    currentPage.value + props.maxPageCountSingle + leftOverflowAmount,
  );
  for (let i = minPage; i <= maxPage; i++) {
    pages.push(i);
  }
  return pages;
});
</script>

<template>
  <div class="p-paginator">
    <template v-for="(page, index) in shownPages" :key="index">
      <Button
        :severity="page === currentPage ? 'primary' : 'secondary'"
        :variant="page === currentPage ? 'filled' : 'text'"
        icon-only
        @click="currentPage = page"
        ><span class="w-6 h-6">{{ page }}</span></Button
      >
    </template>
  </div>
</template>
