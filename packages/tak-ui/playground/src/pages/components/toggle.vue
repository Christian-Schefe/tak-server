<script setup lang="ts">
import { Toggle } from '@tak-ui-lib/components';
import Page from '../../components/Page.vue';
</script>
<template>
  <Page>
    <h1>Toggles</h1>
    <h2>Normal Toggles</h2>
    <Toggle />
    <Toggle :model-value="true" />
    <h2>Disabled Toggles</h2>
    <Toggle disabled />
    <Toggle :model-value="true" disabled />
  </Page>
</template>
