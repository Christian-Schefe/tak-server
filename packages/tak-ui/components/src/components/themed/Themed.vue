<script setup lang="ts">
import { computed, type StyleValue } from 'vue';
import { getThemeStyles, type Theme } from '../../theme';

const props = defineProps<{
  theme: Theme;
  isDark: boolean;
}>();

const themeStyles = computed<StyleValue>(() => {
  return getThemeStyles(props.theme, props.isDark);
});
</script>
<template>
  <div class="p-themed" :style="themeStyles"><slot /></div>
</template>
<style lang="css" scoped>
.p-themed {
  background-color: var(--p-color-background);
  color: var(--p-color-text);
}
</style>
