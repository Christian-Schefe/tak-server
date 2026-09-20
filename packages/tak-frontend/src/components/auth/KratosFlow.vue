<script setup lang="ts">
import { useKratosFlow, type KratosFlowType } from '@/features/auth.ts';
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
</template>
