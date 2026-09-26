<script setup lang="ts">
import { type CreateSeekPayload } from '@/api/seek';
import { getDefaultReserve } from '@/tak-core';
import { zodFormValidator } from '@/utils/forms';
import {
  Button,
  createFormContext,
  Dialog,
  Form,
  InputNumber,
  Select,
  Slider,
} from '@tak-ui-lib/components';
import { ref, watch } from 'vue';
import { z } from 'zod';

const visible = defineModel<boolean>({ required: true });

const emit = defineEmits<{
  create: [CreateSeekPayload];
}>();

const createSeekFormSchema = z.object({
  boardSize: z.int().min(3).max(8),
  isRated: z.boolean(),
  contingentMinutes: z
    .int()
    .min(1)
    .max(60 * 24)
    .optional()
    .nullable(),
  incrementSeconds: z
    .int()
    .min(0)
    .max(60 * 60)
    .optional()
    .nullable(),
  pieces: z.int().min(1).max(200).optional().nullable(),
  capstones: z.int().min(0).max(20).optional().nullable(),
  halfKomi: z.int().min(0).max(20),
  opening: z.enum(['swap', 'noSwap', 'doubleStack']),
  color: z.enum(['random', 'white', 'black']),
});

type CreateSeekFormData = z.infer<typeof createSeekFormSchema>;

function onSubmit(formData: CreateSeekFormData) {
  const defaultReserve = getDefaultReserve(formData.boardSize);
  const payload: CreateSeekPayload = {
    gameSettings: {
      boardSize: formData.boardSize,
      pieces: formData.pieces ?? defaultReserve.pieces,
      capstones: formData.capstones ?? defaultReserve.capstones,
      halfKomi: formData.halfKomi,
      opening: formData.opening,
      timeSettings: {
        type: 'realtime',
        contingentMs: (formData.contingentMinutes ?? 15) * 60 * 1000,
        incrementMs: (formData.incrementSeconds ?? 10) * 1000,
        extra: null,
      },
    },
    isRated: formData.isRated,
    color: formData.color,
  };
  emit('create', payload);
  visible.value = false;
}

const initialFormValues: Partial<CreateSeekFormData> = {
  boardSize: 6,
  isRated: true,
  contingentMinutes: 15,
  incrementSeconds: 10,
  pieces: undefined,
  capstones: undefined,
  halfKomi: 0,
  opening: 'swap',
  color: 'random',
};

const halfKomiValue = ref(0);
const boardSizeValue = ref(6);

const validator = zodFormValidator(createSeekFormSchema);

const formCtx = createFormContext(() => initialFormValues);
watch(visible, (newVisible) => {
  if (!newVisible) {
    formCtx.value.reset();
  }
});
</script>
<template>
  <Dialog v-model:visible="visible" header="Create Seek">
    <Form v-model="formCtx" :validator="validator" @submit="onSubmit">
      <div class="w-full flex flex-col gap-2">
        <Select
          :model-value="'random'"
          name="color"
          :options="[
            { label: 'Random', value: 'random' },
            { label: 'White', value: 'white' },
            { label: 'Black', value: 'black' },
          ]"
          label="Play as"
        />
        <Select
          :model-value="boardSizeValue"
          name="boardSize"
          :options="[
            { label: '3x3', value: 3 },
            { label: '4x4', value: 4 },
            { label: '5x5', value: 5 },
            { label: '6x6', value: 6 },
            { label: '7x7', value: 7 },
            { label: '8x8', value: 8 },
          ]"
          label="Board Size"
          @update:model-value="boardSizeValue = $event ?? 6"
        />

        <p class="text-sm text-muted-color-emphasis text-nowrap mt-3">Komi</p>
        <div
          class="grid items-center gap-2 justify-start"
          :style="{ gridTemplateColumns: '1fr 90px' }"
        >
          <Slider v-model="halfKomiValue" name="halfKomi" :min="0" :max="20" :step="1" />
          <p class="text-right border-surface font-mono">{{ halfKomiValue * 0.5 }} komi</p>
        </div>

        <div class="w-full grid grid-cols-2 gap-2">
          <div class="flex flex-col gap-2">
            <InputNumber
              input-id="contingentMinutes"
              name="contingentMinutes"
              placeholder="15"
              label="Contingent Time (minutes)"
            />
            <p v-if="formCtx.errors.contingentMinutes">
              {{ formCtx.errors.contingentMinutes }}
            </p>
          </div>
          <div class="flex flex-col gap-2">
            <InputNumber
              input-id="incrementSeconds"
              name="incrementSeconds"
              placeholder="10"
              label="Increment (seconds)"
            />
            <p v-if="formCtx.errors.incrementSeconds">
              {{ formCtx.errors.incrementSeconds }}
            </p>
          </div>
        </div>

        <div class="w-full grid grid-cols-2 gap-2">
          <div class="flex flex-col gap-2">
            <InputNumber
              input-id="pieces"
              name="pieces"
              :placeholder="getDefaultReserve(boardSizeValue).pieces.toString()"
              label="Pieces"
            />
            <p v-if="formCtx.errors.pieces">
              {{ formCtx.errors.pieces }}
            </p>
          </div>
          <div class="flex flex-col gap-2">
            <InputNumber
              input-id="capstones"
              name="capstones"
              :placeholder="getDefaultReserve(boardSizeValue).capstones.toString()"
              label="Capstones"
            />
            <p v-if="formCtx.errors.capstones">
              {{ formCtx.errors.capstones }}
            </p>
          </div>
        </div>
        <Select
          :model-value="'swap'"
          name="opening"
          :options="[
            { label: 'Swap', value: 'swap' },
            { label: 'No Swap', value: 'noSwap' },
            { label: 'Double Stack', value: 'doubleStack' },
          ]"
          label="Opening"
        />
        <Select
          :model-value="true"
          name="isRated"
          :options="[
            { label: 'Rated', value: true },
            { label: 'Unrated', value: false },
          ]"
          label="Rated"
        />
        <div class="col-span-2 w-full flex justify-end gap-2 pt-12">
          <Button label="Cancel" severity="secondary" @click="visible = false" />
          <Button type="submit" label="Create Seek" />
        </div>
      </div>
    </Form>
  </Dialog>
</template>
