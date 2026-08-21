<script setup lang="ts">
import { Icon, Select } from '@tak-ui-lib/components';
import Page from '../../components/Page.vue';
</script>
<template>
  <Page>
    <h1>Select</h1>
    <Select
      :model-value="undefined"
      label="Options with long text"
      :options="[
        { label: 'Option 1 Option 1 Option 1 Option 1', value: 1 },
        { label: 'Option 2', value: 2 },
        { label: 'Option 3', value: 3 },
      ]"
      ><template #icon-prepend>
        <Icon name="home" />
      </template>
    </Select>
    <Select
      :model-value="undefined"
      placeholder="Select an option"
      :options="[
        { label: 'Option 1', value: 1 },
        { label: 'Option 2', value: 2 },
        { label: 'Option 3', value: 3 },
      ]"
      :allow-empty-with-default="{ default: undefined }"
    />
    <Select
      :model-value="undefined"
      placeholder="Select an option"
      :options="Array.from({ length: 100 }, (_, i) => ({ label: `Option ${i + 1}`, value: i + 1 }))"
      :allow-empty-with-default="{ default: undefined }"
    />
  </Page>
</template>
