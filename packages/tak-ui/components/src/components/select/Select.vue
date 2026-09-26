<script setup lang="ts" generic="T, AllowEmpty extends boolean = false">
import { type Placement } from '@floating-ui/vue';
import { computed, ref, useTemplateRef } from 'vue';
import { Button } from '../button';
import { Dropdown } from '../dropdown';
import { Icon } from '../icon';
import { LabelField } from '..';
import { useFormValue } from '../../form';

// eslint-disable-next-line @typescript-eslint/no-redundant-type-constituents
type MaybeT = AllowEmpty extends true ? T | undefined : T;

const value = defineModel<MaybeT>({ required: true });
const props = withDefaults(
  defineProps<{
    options: readonly { value: T; label: string }[];
    allowEmpty?: AllowEmpty | undefined;
    placeholder?: string;
    label?: string | undefined;
    placement?: Placement;
    disabled?: boolean;
    name?: string | undefined;
    cmp?: (a: T, b: T) => boolean;
  }>(),
  {
    allowEmpty: undefined,
    placeholder: 'Placeholder',
    placement: 'bottom-start',
    label: undefined,
    disabled: false,
    name: undefined,
    cmp: (a, b) => a === b,
  },
);

const dropdownVisible = ref(false);

const currentOption = computed(() => {
  const val = value.value;
  if (val === undefined) {
    return undefined;
  }
  return props.options.find((option) => props.cmp(option.value, val));
});

const optionLabel = computed(() => {
  return currentOption.value ? currentOption.value.label : null;
});

function onSelectOption(optionValue: T) {
  if (props.disabled) {
    return;
  }
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-type-assertion
  value.value = (
    value.value === optionValue && props.allowEmpty === true ? undefined : optionValue
  ) as MaybeT;
  dropdownVisible.value = false;
}

function onToggleDropdown() {
  if (props.disabled) {
    return;
  }
  dropdownVisible.value = !dropdownVisible.value;
}
const reference = useTemplateRef<HTMLElement | null>('reference');

useFormValue(value, () => props.name);
</script>
<template>
  <LabelField :label="label" :disabled="disabled" :focused="dropdownVisible">
    <div
      ref="reference"
      class="p-select"
      :class="{
        'p-select-open': dropdownVisible,
        'p-select-empty': optionLabel === null,
        'p-select-disabled': disabled,
      }"
      @click="onToggleDropdown"
    >
      <p class="p-select-optionlabel">
        {{ optionLabel !== null ? optionLabel : placeholder }}
      </p>
    </div>
    <template v-if="$slots['icon-prepend']" #icon-prepend>
      <slot name="icon-prepend" />
    </template>
    <template #icon-append>
      <Icon class="p-select-icon" name="chevron-down" />
    </template>
  </LabelField>
  <Dropdown v-model="dropdownVisible" :reference="reference">
    <div class="p-select-dropdown">
      <div class="p-select-dropdown-inner">
        <Button
          v-for="(option, index) in props.options"
          :key="index"
          :severity="value === option.value ? 'primary' : 'secondary'"
          variant="text"
          :label="option.label"
          @click="onSelectOption(option.value)"
        />
      </div>
    </div>
  </Dropdown>
</template>
