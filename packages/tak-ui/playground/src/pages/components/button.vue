<script setup lang="ts">
import { Button, ButtonGroup } from '@tak-ui-lib/components';
import Page from '../../components/Page.vue';
import { LuAlarmClock } from 'vue-icons-plus/lu';

const variants = ['filled', 'text', 'outlined'] as const;
const severities = ['primary', 'secondary'] as const;
</script>
<template>
  <Page>
    <h1>Buttons</h1>
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
  </Page>
</template>
