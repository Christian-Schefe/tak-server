<script setup lang="ts">
import { Button, InputText, Slider, Tooltip } from '@tak-ui-lib/components';
import Page from '../../components/Page.vue';
import { ref } from 'vue';

const tooltipSliderValue = ref(60);
</script>
<template>
  <Page>
    <h1>Tooltips</h1>
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
    <Tooltip :activation="tooltipSliderValue > 50" placement="bottom">
      <Slider v-model="tooltipSliderValue" />
      <template #content>
        <p>This tooltip shows if the slider value is more than 50.</p>
        <p>You can put any content here.</p>
      </template>
    </Tooltip>
  </Page>
</template>
