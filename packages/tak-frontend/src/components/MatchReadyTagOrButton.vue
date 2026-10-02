<script setup lang="ts">
import { Button, Tag } from '@tak-ui-lib/components';

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
    :severity="isOnline ? (isReady ? 'primary' : 'danger') : 'secondary'"
    @click="$emit('toggleReady')"
  />
  <Tag
    v-else
    :label="isOnline ? (isReady ? 'Ready' : 'Not Ready') : 'Offline'"
    :severity="isOnline ? (isReady ? 'primary' : 'danger') : 'secondary'"
  />
</template>
