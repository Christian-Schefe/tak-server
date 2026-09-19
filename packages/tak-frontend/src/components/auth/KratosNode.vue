<script setup lang="ts">
import type { UiNode } from '@ory/client';
import { Button, InputText } from '@tak-ui-lib/components';
import { computed } from 'vue';
import { RouterLink } from 'vue-router';

const props = defineProps<{
  node: UiNode;
}>();

const autocomplete = computed(() => {
  if (props.node.attributes.node_type !== 'input') {
    return undefined;
  }
  const name = props.node.attributes.name;
  if (name === 'identifier') {
    return props.node.attributes.autocomplete || 'username';
  }
  return props.node.attributes.autocomplete;
});
</script>

<template>
  <Button
    v-if="node.attributes.node_type === 'a'"
    :as="{ component: RouterLink, props: { to: node.attributes.href } }"
    :label="node.meta.label?.text"
  />
  <Button
    v-else-if="
      node.attributes.node_type === 'input' &&
      (node.attributes.type === 'submit' || node.attributes.type === 'button')
    "
    :name="node.attributes.name"
    :type="node.attributes.type"
    :value="node.attributes.value"
    :label="node.meta.label?.text"
  />
  <InputText
    v-else-if="node.attributes.node_type === 'input' && node.attributes.type === 'hidden'"
    :name="node.attributes.name"
    :type="node.attributes.type"
    :autocomplete="autocomplete"
    :disabled="node.attributes.disabled"
    :required="node.attributes.required ?? false"
    hidden
  />
  <InputText
    v-else-if="node.attributes.node_type === 'input' && node.attributes.type === 'password'"
    :input-id="`kratos-input-${node.attributes.name}`"
    :name="node.attributes.name"
    :type="node.attributes.type"
    :disabled="node.attributes.disabled"
    :required="node.attributes.required ?? false"
    :label="`${node.meta.label?.text}${node.attributes.required === true ? ' *' : ''}`"
  />
  <InputText
    v-else-if="node.attributes.node_type === 'input'"
    :input-id="`kratos-input-${node.attributes.name}`"
    :name="node.attributes.name"
    :type="node.attributes.type"
    :autocomplete="autocomplete"
    :disabled="node.attributes.disabled"
    :required="node.attributes.required ?? false"
    :label="`${node.meta.label?.text}${node.attributes.required === true ? ' *' : ''}`"
  />
  <p v-else-if="node.attributes.node_type === 'text'">{{ node.attributes.text }}</p>
</template>
