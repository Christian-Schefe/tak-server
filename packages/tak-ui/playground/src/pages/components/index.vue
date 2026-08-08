<script setup lang="ts">
import {
  Button,
  Card,
  Dialog,
  InputNumber,
  InputText,
  Select,
  Slider,
  Toggle,
  Tooltip,
} from '@tak-ui-lib/components';
import { ref } from 'vue';
import { LuAlarmClock, LuX } from 'vue-icons-plus/lu';
import Page from '../../components/Page.vue';
import ButtonGroup from '@tak-ui-lib/components/src/components/buttongroup/ButtonGroup.vue';

const inputSliderValue = ref(50);

const variants = ['filled', 'text', 'outlined'] as const;
const severities = ['primary', 'secondary'] as const;

const tooltipSliderValue = ref(60);
const dialogVisible = ref(false);

const outerDialogVisible = ref(false);
const innerDialogVisible = ref(false);
const inputTextValue = ref('');
</script>
<template>
  <Page>
    <h1>Buttons</h1>
    <Card>
      <h2>Text Buttons</h2>
      <div class="flex gap-2 items-center flex-wrap">
        <template v-for="variant in variants" :key="variant">
          <template v-for="severity in severities" :key="severity">
            <Button :variant="variant" :severity="severity" :label="`${severity} ${variant}`" />
          </template>
        </template>
      </div>
      <h2>Icon Buttons</h2>
      <div class="flex gap-2 items-center flex-wrap">
        <template v-for="variant in variants" :key="variant">
          <template v-for="severity in severities" :key="severity">
            <Button :variant="variant" :severity="severity" icon-only>
              <LuAlarmClock />
            </Button>
          </template>
        </template>
      </div>
      <h2>Mixed Buttons</h2>
      <div class="flex gap-2 items-center flex-wrap">
        <template v-for="variant in variants" :key="variant">
          <template v-for="severity in severities" :key="severity">
            <Button :variant="variant" :severity="severity" :label="`${severity} ${variant}`">
              <template #icon><LuAlarmClock /></template>
            </Button>
          </template>
        </template>
      </div>
      <h2>Disabled Buttons</h2>
      <div class="flex gap-2 items-center flex-wrap">
        <template v-for="variant in variants" :key="variant">
          <template v-for="severity in severities" :key="severity">
            <Button
              :variant="variant"
              :severity="severity"
              disabled
              :label="`${severity} ${variant}`"
            />
          </template>
        </template>
      </div>
      <h2>Custom Buttons</h2>
      <div class="flex gap-2 items-center flex-wrap">
        <Button>
          <span class="text-lg">Custom<br />Template</span>
        </Button>
        <Button
          :as="{ component: 'a', props: { href: 'https://example.com', target: '_blank' } }"
          label="Link"
        />
      </div>
      <h2>Button Group</h2>
      <div class="flex gap-2 items-center flex-wrap">
        <ButtonGroup>
          <Button label="Button 1" />
          <Button label="Button 2" />
          <Button label="Button 3" />
        </ButtonGroup>
      </div>
    </Card>
    <h1>Sliders</h1>
    <Card>
      <h2>Normal Sliders</h2>
      <Slider :model-value="30" />
      <h2>Stepped Sliders</h2>
      <Slider :model-value="18" :min="0" :max="100" :step="1" />
      <Slider :model-value="40" :min="0" :max="100" :step="10" />
      <Slider :model-value="50" :min="0" :max="100" :step="50" />
      <h2>Disabled Sliders</h2>
      <Slider :model-value="20" :min="0" :max="100" disabled />
      <h2>Controlled Sliders</h2>
      <div class="grid gap-2 items-center" :style="{ gridTemplateColumns: '1fr 80px' }">
        <Slider v-model="inputSliderValue" :min="0" :max="100" :step="1" />
        <InputNumber v-model="inputSliderValue" />
      </div>
    </Card>
    <h1>Select</h1>
    <Card>
      <Select
        :model-value="undefined"
        label="Select an option"
        :options="[
          { label: 'Option 1 Option 1 Option 1 Option 1', value: 1 },
          { label: 'Option 2', value: 2 },
          { label: 'Option 3', value: 3 },
        ]"
      />
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
    <h1>Toggles</h1>
    <Card>
      <h2>Normal Toggles</h2>
      <Toggle />
      <Toggle :model-value="true" />
      <h2>Disabled Toggles</h2>
      <Toggle disabled />
      <Toggle :model-value="true" disabled />
    </Card>
    <h1>Input Text</h1>
    <Card>
      <div class="flex gap-2 items-center flex-wrap">
        <InputText placeholder="Enter text" label="Label" input-id="input1" />
        <InputText
          v-model="inputTextValue"
          placeholder="Enter text"
          label="Label"
          input-id="input2"
          support-text="Support text"
        />
        <InputText
          v-model="inputTextValue"
          placeholder="Enter text"
          label="Label"
          input-id="input3"
        />
        <InputText v-model="inputTextValue" placeholder="Enter text" input-id="input4">
          <LuX v-if="inputTextValue" @click="inputTextValue = ''" />
        </InputText>
        <InputText
          v-model="inputTextValue"
          placeholder="Enter text"
          label="Label"
          input-id="input5"
        >
          <LuX v-if="inputTextValue" @click="inputTextValue = ''" />
        </InputText>
      </div>
    </Card>
    <div class="h-200" />
  </Page>
</template>
