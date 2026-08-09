<script setup lang="ts">
import { Slider, InputNumber } from '@tak-ui-lib/components';
import Page from '../../components/Page.vue';
import { ref } from 'vue';

const inputSliderValue = ref(50);
</script>
<template>
  <Page>
    <h1>Sliders</h1>
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
  </Page>
</template>
