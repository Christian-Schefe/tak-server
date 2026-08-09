<script setup lang="ts">
import {
  Button,
  Card,
  Dialog,
  Icon,
  InputText,
  Select,
  Slider,
  Tooltip,
} from '@tak-ui-lib/components';
import { ref } from 'vue';
import Page from '../../components/Page.vue';

const tooltipSliderValue = ref(60);
const dialogVisible = ref(false);

const outerDialogVisible = ref(false);
const innerDialogVisible = ref(false);
</script>
<template>
  <Page>
    <h1>Select</h1>
    <Card>
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
        :options="
          Array.from({ length: 100 }, (_, i) => ({ label: `Option ${i + 1}`, value: i + 1 }))
        "
        :allow-empty-with-default="{ default: undefined }"
      />
    </Card>
    <h1>Dialogs</h1>
    <Card>
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
    </Card>
    <h1>Tooltips</h1>
    <Card>
      <h2>Tooltip on hover</h2>
      <div class="flex gap-2 items-center flex-wrap">
        <template
          v-for="placement in [
            'top',
            'top-start',
            'top-end',
            'bottom',
            'bottom-start',
            'bottom-end',
            'left',
            'left-start',
            'left-end',
            'right',
            'right-start',
            'right-end',
          ]"
          :key="placement"
        >
          <Tooltip :placement="placement" class="w-fit">
            <Button :label="`${placement}`" />
            <template #content>
              <div class="p-2">
                <p>This is a tooltip.</p>
                <p>You can put any content here.</p>
              </div>
            </template>
          </Tooltip>
        </template>
      </div>
      <h2>Tooltip on focus</h2>
      <Tooltip activation="focus" placement="left" class="w-fit">
        <InputText placeholder="Focus on me" />
        <template #content>
          <p>This is a tooltip.</p>
          <p>You can put any content here.</p>
        </template>
      </Tooltip>
      <h2>Controlled tooltip</h2>
      <Tooltip :activation="tooltipSliderValue > 50" placement="left">
        <Slider v-model="tooltipSliderValue" />
        <template #content>
          <p>This tooltip shows if the slider value is more than 50.</p>
          <p>You can put any content here.</p>
        </template>
      </Tooltip>
    </Card>
    <div class="h-200" />
  </Page>
</template>
