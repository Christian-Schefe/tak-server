<script setup lang="ts">
import { useSanitizedFlagSVG } from '@/utils/flags';

const props = defineProps<{
  country: string | undefined;
}>();
const flagSVG = useSanitizedFlagSVG(() => props.country);
</script>
<template>
  <!-- eslint-disable vue/no-v-html -->
  <div v-if="flagSVG" class="w-6 rounded-xs overflow-hidden" v-html="flagSVG"></div>
</template>
