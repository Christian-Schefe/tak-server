<script setup lang="ts">
import type { UiContainer, UiNode } from '@ory/client';
import { computed } from 'vue';
import KratosNode from './KratosNode.vue';
import { Form } from '@tak-ui-lib/components';

const emit = defineEmits<{
  (e: 'submit', value: unknown): void;
}>();

interface NodeWithId {
  node: UiNode;
  id: string;
}

interface NodeGroup {
  nodes: NodeWithId[];
  initialValues: Record<string, string>;
}

const props = defineProps<{
  ui: UiContainer;
}>();

const nodes = computed(() => {
  const groups: Record<string, NodeGroup> = props.ui.nodes
    .map((node) => node.group)
    .reduce<Record<string, NodeGroup>>((acc, group) => {
      if (group !== 'default') {
        acc[group] = {
          nodes: [],
          initialValues: {},
        };
      }
      return acc;
    }, {});
  props.ui.nodes.forEach((node) => {
    const mappedNode = {
      node,
      id: `kratos-node-${crypto.randomUUID()}`,
    };
    const groupsForNode = node.group === 'default' ? Object.values(groups) : [groups[node.group]];
    for (const group of groupsForNode) {
      if (!group) continue;
      group.nodes.push(mappedNode);
      if (node.attributes.node_type === 'input') {
        group.initialValues[node.attributes.name] = node.attributes.value;
      }
    }
  });
  return Object.entries(groups);
});

function submit(data: Record<string, unknown>) {
  emit('submit', data);
}
</script>

<template>
  <div class="flex flex-col items-stretch gap-4">
    <Form
      v-for="[group, groupNodes] in nodes"
      :key="group"
      :initial-values="groupNodes.initialValues"
      :validator="(data) => ({ type: 'success', data })"
      @submit="submit"
    >
      <div class="flex flex-col items-stretch gap-2">
        <KratosNode v-for="node in groupNodes.nodes" :key="node.id" :node="node.node"></KratosNode>
      </div>
    </Form>
  </div>
  <div v-if="ui.messages && ui.messages.length > 0" class="flex flex-col items-stretch gap-2">
    <Message
      v-for="item in ui.messages"
      :key="item.id"
      :severity="item.type === 'error' ? 'error' : 'info'"
      size="small"
      variant="simple"
      >{{ item.text }}</Message
    >
  </div>
</template>
