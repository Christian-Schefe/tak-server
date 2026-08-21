<script setup lang="ts">
import { Button, Select, Dialog } from '@tak-ui-lib/components';
import Page from '../../components/Page.vue';
import { ref } from 'vue';

const dialogVisible = ref(false);

const outerDialogVisible = ref(false);
const innerDialogVisible = ref(false);
</script>
<template>
  <Page>
    <h1>Dialogs</h1>
    <div class="flex gap-2 items-center flex-wrap">
      <Button label="Open Dialog" @click="dialogVisible = true" />
      <Dialog v-model:visible="dialogVisible" :header="'Hello'.repeat(30)">
        <Select
          :model-value="undefined"
          :options="[
            { label: 'Option 1 Option 1 Option 1 Option 1', value: 1 },
            { label: 'Option 2', value: 2 },
            { label: 'Option 3', value: 3 },
          ]"
        />
        <p v-for="i in 70" :key="i">Content {{ i }}</p>
        <template #footer>
          <Button label="Cancel" @click="dialogVisible = false" />
          <Button label="Confirm" severity="primary" @click="dialogVisible = false" />
        </template>
      </Dialog>
      <Button label="Open Nested Dialogs" @click="outerDialogVisible = true" />
      <Dialog v-model:visible="outerDialogVisible" :header="'Outer Dialog'">
        <p>This is the outer dialog.</p>
        <Button label="Open Inner Dialog" @click="innerDialogVisible = true" />
        <Dialog v-model:visible="innerDialogVisible" :header="'Inner Dialog'">
          <p>This is the inner dialog.</p>
          <Select
            :model-value="undefined"
            :options="[
              { label: 'Option 1 Option 1 Option 1 Option 1', value: 1 },
              { label: 'Option 2', value: 2 },
              { label: 'Option 3', value: 3 },
            ]"
          />
        </Dialog>
      </Dialog>
    </div>
  </Page>
</template>
