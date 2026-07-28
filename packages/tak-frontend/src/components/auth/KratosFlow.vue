<script setup lang="ts">
import { useKratosFlow, type KratosFlowType } from '@/features/auth.ts';
import Skeleton from 'primevue/skeleton';
import KratosForm from './KratosForm.vue';

const props = withDefaults(
  defineProps<{
    flowType: KratosFlowType;
    initialFlowId?: string;
  }>(),
  {
    initialFlowId: undefined,
  },
);

const { flow, submitFlow } = useKratosFlow(props.flowType, props.initialFlowId);

async function onSubmit(data: unknown) {
  if (flow.value) {
    const result = await submitFlow(props.flowType, flow.value.id, data);
    if (result) {
      flow.value = result;
    }
  }
}
</script>

<template>
  <KratosForm v-if="flow" :ui="flow.ui" @submit="(data) => void onSubmit(data)"></KratosForm>
  <div v-else class="flex flex-col gap-4 w-full h-60">
    <Skeleton width="100%" border-radius="4px" class="flex-1" />
    <Skeleton width="100%" border-radius="4px" class="flex-1" />
    <Skeleton width="100%" border-radius="4px" class="flex-1" />
  </div>
</template>
