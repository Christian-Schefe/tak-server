<script setup lang="ts">
import type { UiNode } from '@ory/client';
import Button from 'primevue/button';
import InputText from 'primevue/inputtext';
import Password from 'primevue/password';
import IftaLabel from 'primevue/iftalabel';
import { computed } from 'vue';

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
  <Button v-if="node.attributes.node_type === 'a'" v-slot="slotProps" as-child fluid>
    <RouterLink v-ripple :class="slotProps.class" :to="node.attributes.href">
      {{ node.meta.label?.text }}
    </RouterLink>
  </Button>
  <Button
    v-else-if="
      node.attributes.node_type === 'input' &&
      (node.attributes.type === 'submit' || node.attributes.type === 'button')
    "
    :name="node.attributes.name"
    :type="node.attributes.type"
    :value="node.attributes.value"
    :label="node.meta.label?.text"
    fluid
  />
  <InputText
    v-else-if="node.attributes.node_type === 'input' && node.attributes.type === 'hidden'"
    :name="node.attributes.name"
    :type="node.attributes.type"
    :autocomplete="autocomplete"
    :disabled="node.attributes.disabled"
    :required="node.attributes.required ?? false"
    hidden
    fluid
  />
  <IftaLabel
    v-else-if="node.attributes.node_type === 'input' && node.attributes.type === 'password'"
  >
    <Password
      :input-id="`kratos-input-${node.attributes.name}`"
      :name="node.attributes.name"
      :type="node.attributes.type"
      :disabled="node.attributes.disabled"
      :required="node.attributes.required ?? false"
      :feedback="false"
      toggle-mask
      :pt="{
        pcInputText: {
          root: {
            autocomplete: autocomplete,
          },
        },
      }"
      fluid
    />
    <label :for="`kratos-input-${node.attributes.name}`"
      >{{ node.meta.label?.text }}{{ node.attributes.required === true ? ' *' : '' }}</label
    >
  </IftaLabel>
  <IftaLabel v-else-if="node.attributes.node_type === 'input'">
    <InputText
      :id="`kratos-input-${node.attributes.name}`"
      :name="node.attributes.name"
      :type="node.attributes.type"
      :autocomplete="autocomplete"
      :disabled="node.attributes.disabled"
      :required="node.attributes.required ?? false"
      fluid
    />
    <label :for="`kratos-input-${node.attributes.name}`"
      >{{ node.meta.label?.text }}{{ node.attributes.required === true ? ' *' : '' }}</label
    >
  </IftaLabel>
  <p v-else-if="node.attributes.node_type === 'text'">{{ node.attributes.text }}</p>
</template>
