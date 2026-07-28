<script setup lang="ts">
import Button from 'primevue/button';
import Tag from 'primevue/tag';

defineProps<{
  isOnline: boolean;
  isReady: boolean;
  isButton: boolean;
}>();
defineEmits<{
  toggleReady: [];
}>();
</script>
<template>
  <Button
    v-if="isButton"
    :label="isOnline ? (isReady ? 'Ready' : 'Not Ready') : 'Offline'"
    :severity="isOnline ? (isReady ? 'success' : 'danger') : 'secondary'"
    @click="$emit('toggleReady')"
  />
  <Tag
    v-else
    :value="isOnline ? (isReady ? 'Ready' : 'Not Ready') : 'Offline'"
    :severity="isOnline ? (isReady ? 'success' : 'danger') : 'secondary'"
  />
</template>
