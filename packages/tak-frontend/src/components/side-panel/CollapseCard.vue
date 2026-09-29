<script setup lang="ts">
import { Button, Card } from '@tak-ui-lib/components';
import { LuChevronDown } from 'vue-icons-plus/lu';

const expanded = defineModel<boolean>({ default: true });
withDefaults(
  defineProps<{
    title: string;
    grow?: boolean;
  }>(),
  { grow: false },
);
</script>
<template>
  <Card :class="`flex flex-col ${grow && expanded ? 'grow' : ''}`">
    <div class="flex gap-2 items-center">
      <Button
        icon-only
        size="small"
        variant="text"
        severity="secondary"
        @click="expanded = !expanded"
      >
        <LuChevronDown
          size="1lh"
          :class="`transition-transform duration-200 ${expanded ? 'rotate-0' : '-rotate-90'}`"
        />
      </Button>
      <h2>
        {{ title }}
      </h2>
    </div>
    <template v-if="expanded">
      <slot />
    </template>
  </Card>
</template>
