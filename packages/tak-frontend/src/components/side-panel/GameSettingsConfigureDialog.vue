<script setup lang="ts">
import { getDefaultReserve, type TakBaseGameSettings } from '@/tak-core';
import { zodFormValidator } from '@/utils/forms';
import { Button, Dialog, Form, Select, useFormContext } from '@tak-ui-lib/components';
import { z } from 'zod';

const visible = defineModel<boolean>({ required: true });

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

function onSubmit(formData: ConfigureFormData) {
  const defaultReserve = getDefaultReserve(formData.boardSize);
  const settings: TakBaseGameSettings = {
    boardSize: formData.boardSize,
    reserve: {
      pieces: formData.pieces ?? defaultReserve.pieces,
      capstones: formData.capstones ?? defaultReserve.capstones,
    },
    halfKomi: formData.halfKomi,
    opening: formData.opening,
  };
  emit('apply', settings);
}

const formCtx = useFormContext({
  boardSize: 6,
  pieces: undefined,
  capstones: undefined,
  halfKomi: 4,
  opening: 'swap',
});
const formValidator = zodFormValidator(configureFormSchema);
</script>
<template>
  <Dialog v-model:visible="visible" header="Configure Analysis">
    <Form
      v-model="formCtx"
      class="flex flex-col gap-2"
      :validator="formValidator"
      @submit="onSubmit"
    >
      <Select
        :model-value="6"
        name="boardSize"
        :options="[
          { label: '3x3', value: 3 },
          { label: '4x4', value: 4 },
          { label: '5x5', value: 5 },
          { label: '6x6', value: 6 },
          { label: '7x7', value: 7 },
          { label: '8x8', value: 8 },
        ]"
      >
      </Select>
      <Select
        :model-value="4"
        name="halfKomi"
        :options="[
          { label: '0 komi', value: 0 },
          { label: '2 komi', value: 4 },
        ]"
      >
      </Select>
      <Select
        model-value="swap"
        name="opening"
        :options="[
          { label: 'Swap', value: 'swap' },
          { label: 'No Swap', value: 'noSwap' },
          { label: 'Double Stack', value: 'doubleStack' },
        ]"
      />
      <Button type="submit" label="Apply"></Button>
    </Form>
  </Dialog>
</template>
