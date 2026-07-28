<script setup lang="ts">
import { getDefaultReserve, type TakBaseGameSettings } from '@/tak-core';
import { Form, type FormSubmitEvent } from '@primevue/forms';
import Button from 'primevue/button';
import SelectButton from 'primevue/selectbutton';
import { ref } from 'vue';
import { z } from 'zod';

const emit = defineEmits<{
  (e: 'apply', settings: TakBaseGameSettings): void;
}>();

const configureFormSchema = z.object({
  boardSize: z.int().min(3).max(8),
  pieces: z.int().min(1).max(200).optional().nullable(),
  capstones: z.int().min(0).max(20).optional().nullable(),
  halfKomi: z.int().min(0).max(20),
  opening: z.enum(['swap', 'noSwap', 'doubleStack']),
});

type ConfigureFormData = z.infer<typeof configureFormSchema>;

function onSubmit(event: FormSubmitEvent) {
  const formData = configureFormSchema.safeParse(event.values);
  if (formData.success) {
    const defaultReserve = getDefaultReserve(formData.data.boardSize);
    const settings: TakBaseGameSettings = {
      boardSize: formData.data.boardSize,
      reserve: {
        pieces: formData.data.pieces ?? defaultReserve.pieces,
        capstones: formData.data.capstones ?? defaultReserve.capstones,
      },
      halfKomi: formData.data.halfKomi,
      opening: formData.data.opening,
    };
    emit('apply', settings);
  } else {
    console.error('Invalid form data', formData.error);
  }
}

const initialValues = ref<ConfigureFormData>({
  boardSize: 5,
  pieces: undefined,
  capstones: undefined,
  halfKomi: 4,
  opening: 'swap',
});
</script>
<template>
  <Form class="flex flex-col gap-2" :initial-values="initialValues" @submit="onSubmit">
    <SelectButton
      name="boardSize"
      :allow-empty="false"
      option-label="label"
      option-value="value"
      :options="[
        { label: '3x3', value: 3 },
        { label: '4x4', value: 4 },
        { label: '5x5', value: 5 },
        { label: '6x6', value: 6 },
        { label: '7x7', value: 7 },
        { label: '8x8', value: 8 },
      ]"
      fluid
    >
    </SelectButton>
    <SelectButton
      name="halfKomi"
      :allow-empty="false"
      option-label="label"
      option-value="value"
      :options="[
        { label: '0 komi', value: 0 },
        { label: '2 komi', value: 4 },
      ]"
      fluid
    >
    </SelectButton>
    <SelectButton
      name="opening"
      :allow-empty="false"
      option-label="label"
      option-value="value"
      :options="[
        { label: 'Swap', value: 'swap' },
        { label: 'No Swap', value: 'noSwap' },
        { label: 'Double Stack', value: 'doubleStack' },
      ]"
      fluid
    />
    <Button type="submit" label="Apply" fluid></Button>
  </Form>
</template>
