<script setup lang="ts">
import { useIcon } from '../../icons';

const props = defineProps<{
  name: string;
}>();

const icon = useIcon(() => props.name);
</script>
<template>
  <component :is="icon" v-if="icon" />
  <p v-else>Not found</p>
</template>
