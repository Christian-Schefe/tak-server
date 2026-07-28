<script setup lang="ts">
import { type CreateSeekPayload } from '@/api/seek';
import { getDefaultReserve } from '@/tak-core';
import { Form, type FormSubmitEvent } from '@primevue/forms';
import { zodResolver } from '@primevue/forms/resolvers/zod';
import Button from 'primevue/button';
import Dialog from 'primevue/dialog';
import IftaLabel from 'primevue/iftalabel';
import InputNumber from 'primevue/inputnumber';
import Message from 'primevue/message';
import SelectButton from 'primevue/selectbutton';
import Slider from 'primevue/slider';
import { ref } from 'vue';
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

function onSubmit(event: FormSubmitEvent) {
  const formData = createSeekFormSchema.safeParse(event.values);
  if (formData.success) {
    const defaultReserve = getDefaultReserve(formData.data.boardSize);
    const payload: CreateSeekPayload = {
      gameSettings: {
        boardSize: formData.data.boardSize,
        pieces: formData.data.pieces ?? defaultReserve.pieces,
        capstones: formData.data.capstones ?? defaultReserve.capstones,
        halfKomi: formData.data.halfKomi,
        opening: formData.data.opening,
        timeSettings: {
          type: 'realtime',
          contingentMs: (formData.data.contingentMinutes ?? 15) * 60 * 1000,
          incrementMs: (formData.data.incrementSeconds ?? 10) * 1000,
          extra: null,
        },
      },
      isRated: formData.data.isRated,
      color: formData.data.color,
    };
    emit('create', payload);
  } else {
    console.error('Invalid form data', formData.error);
  }
  visible.value = false;
}
const initialValues = ref<CreateSeekFormData>({
  boardSize: 6,
  isRated: true,
  contingentMinutes: undefined,
  incrementSeconds: undefined,
  pieces: undefined,
  capstones: undefined,
  halfKomi: 4,
  opening: 'swap',
  color: 'random',
});

const resolver = zodResolver(createSeekFormSchema);
</script>
<template>
  <Dialog
    v-model:visible="visible"
    dismissable-mask
    header="Create Seek"
    :draggable="false"
    modal
    :style="{ width: '90vw', maxWidth: '600px' }"
  >
    <Form v-slot="$form" :resolver="resolver" :initial-values="initialValues" @submit="onSubmit">
      <div class="w-full flex flex-col gap-1">
        <p class="text-sm text-muted-color-emphasis text-nowrap mt-3">Play as</p>
        <SelectButton
          name="color"
          :allow-empty="false"
          option-label="label"
          option-value="value"
          :options="[
            { label: 'Random', value: 'random' },
            { label: 'White', value: 'white' },
            { label: 'Black', value: 'black' },
          ]"
          fluid
        />
        <p class="text-sm text-muted-color-emphasis text-nowrap mt-3">Board Size</p>
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
        />

        <p class="text-sm text-muted-color-emphasis text-nowrap mt-3">Komi</p>
        <div
          class="grid items-center gap-2 justify-start p-2"
          :style="{ gridTemplateColumns: '1fr 90px' }"
        >
          <Slider name="halfKomi" :min="0" :max="20" :step="1" />
          <p class="text-right border-surface font-mono">{{ $form.halfKomi?.value * 0.5 }} komi</p>
        </div>

        <p class="text-sm text-muted-color-emphasis text-nowrap mt-3">Time Control</p>
        <div class="w-full grid grid-cols-2 gap-2">
          <div class="flex flex-col gap-2">
            <IftaLabel>
              <InputNumber
                input-id="contingentMinutes"
                name="contingentMinutes"
                placeholder="15"
                fluid
              />
              <label for="contingentMinutes">Contingent Time (minutes)</label>
            </IftaLabel>
            <Message
              v-if="$form.contingentMinutes?.invalid"
              severity="error"
              size="small"
              variant="simple"
              >{{ $form.contingentMinutes.error?.message }}</Message
            >
          </div>
          <div class="flex flex-col gap-2">
            <IftaLabel>
              <InputNumber
                input-id="incrementSeconds"
                name="incrementSeconds"
                placeholder="10"
                fluid
              />
              <label for="incrementSeconds">Increment (seconds)</label>
            </IftaLabel>
            <Message
              v-if="$form.incrementSeconds?.invalid"
              severity="error"
              size="small"
              variant="simple"
              >{{ $form.incrementSeconds.error?.message }}</Message
            >
          </div>
        </div>

        <p class="text-sm text-muted-color-emphasis text-nowrap mt-3">Reserve</p>
        <div class="w-full grid grid-cols-2 gap-2">
          <div class="flex flex-col gap-2">
            <IftaLabel>
              <InputNumber
                input-id="pieces"
                name="pieces"
                :placeholder="getDefaultReserve($form.boardSize?.value).pieces.toString()"
                fluid
              />
              <label for="pieces">Pieces</label>
            </IftaLabel>
            <Message v-if="$form.pieces?.invalid" severity="error" size="small" variant="simple">{{
              $form.pieces.error?.message
            }}</Message>
          </div>
          <div class="flex flex-col gap-2">
            <IftaLabel>
              <InputNumber
                input-id="capstones"
                name="capstones"
                :placeholder="getDefaultReserve($form.boardSize?.value).capstones.toString()"
                fluid
              />
              <label for="capstones">Capstones</label>
            </IftaLabel>
            <Message
              v-if="$form.capstones?.invalid"
              severity="error"
              size="small"
              variant="simple"
              >{{ $form.capstones.error?.message }}</Message
            >
          </div>
        </div>

        <p class="text-sm text-muted-color-emphasis text-nowrap mt-3">Opening</p>
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

        <p class="text-sm text-muted-color-emphasis text-nowrap mt-3">Rated</p>
        <SelectButton
          name="isRated"
          :allow-empty="false"
          option-label="label"
          option-value="value"
          :options="[
            { label: 'Rated', value: true },
            { label: 'Unrated', value: false },
          ]"
          fluid
        />
        <div class="col-span-2 w-full grid grid-cols-2 gap-2 pt-12">
          <Button label="Cancel" severity="secondary" @click="visible = false" />
          <Button type="submit" label="Create Seek" />
        </div>
      </div>
    </Form>
  </Dialog>
</template>
