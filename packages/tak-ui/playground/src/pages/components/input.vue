<script setup lang="ts">
import { InputText } from '@tak-ui-lib/components';
import { ref } from 'vue';
import { LuSearch, LuX } from 'vue-icons-plus/lu';
import Page from '../../components/Page.vue';

const inputTextValue = ref('');
</script>
<template>
  <Page>
    <h1>Input Text</h1>
    <h2>Normal</h2>
    <div class="flex gap-2 items-start flex-wrap">
      <InputText placeholder="Enter text" label="Label" input-id="input1" />
      <InputText
        v-model="inputTextValue"
        placeholder="Enter text"
        label="Label"
        input-id="input2"
        support-text="Support text"
      >
        <template #icon-prepend><LuSearch /></template>
      </InputText>
      <InputText
        v-model="inputTextValue"
        placeholder="Enter text"
        label="Label"
        input-id="input3"
      />
      <InputText v-model="inputTextValue" placeholder="Enter text" input-id="input4">
        <template v-if="inputTextValue" #icon-append>
          <div
            class="cursor-pointer absolute inset-0 z-10 flex items-center justify-center"
            @click="inputTextValue = ''"
          >
            <LuX />
          </div>
        </template>
      </InputText>
      <InputText v-model="inputTextValue" placeholder="Enter text" label="Label" input-id="input5">
        <template v-if="inputTextValue" #icon-append>
          <div
            class="cursor-pointer absolute inset-0 z-10 flex items-center justify-center"
            @click="inputTextValue = ''"
          >
            <LuX />
          </div>
        </template>
      </InputText>
    </div>
    <h2>Disabled</h2>
    <div class="flex gap-2 items-start flex-wrap">
      <InputText placeholder="Enter text" label="Label" input-id="input6" disabled />
      <InputText
        v-model="inputTextValue"
        placeholder="Enter text"
        label="Label"
        input-id="input7"
        support-text="Support text"
        disabled
        ><template #icon-prepend><LuSearch /></template
      ></InputText>
    </div>
  </Page>
</template>
