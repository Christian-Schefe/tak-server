<script setup lang="ts">
import { Badge, Button } from '@tak-ui-lib/components';
import Page from '../../components/Page.vue';
</script>
<template>
  <Page>
    <h1>Badges</h1>
    <Badge>
      <Button>Button with Badge</Button>
    </Badge>
    <Badge label="1">
      <Button>Button with Badge</Button>
    </Badge>
    <Badge label="34">
      <Button>Button with Badge</Button>
    </Badge>
    <Badge label="99+">
      <Button>Button with Badge</Button>
    </Badge>
    <Badge label="new">
      <Button>Button with Badge</Button>
    </Badge>
  </Page>
</template>
