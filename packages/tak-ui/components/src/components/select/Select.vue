<script setup lang="ts" generic="T">
import { type Placement } from '@floating-ui/vue';
import { computed, ref, useTemplateRef } from 'vue';
import { Button } from '../button';
import { Dropdown } from '../dropdown';
import { Icon } from '../icon';

const value = defineModel<T>({ required: true });
const props = withDefaults(
  defineProps<{
    options: { value: T; label: string }[];
    allowEmptyWithDefault?: { default: T } | undefined;
    placeholder?: string;
    label?: string | undefined;
    placement?: Placement;
    disabled?: boolean;
    cmp?: (a: T, b: T) => boolean;
  }>(),
  {
    allowEmptyWithDefault: undefined,
    placeholder: 'Placeholder',
    placement: 'bottom-start',
    label: undefined,
    disabled: false,
    cmp: (a, b) => a === b,
  },
);

const dropdownVisible = ref(false);

const currentOption = computed(() => {
  return props.options.find((option) => {
    return props.cmp(option.value, value.value);
  });
});

const optionLabel = computed(() => {
  return currentOption.value ? currentOption.value.label : null;
});

function onSelectOption(optionValue: T) {
  if (props.disabled) {
    return;
  }
  value.value =
    value.value === optionValue && props.allowEmptyWithDefault !== undefined
      ? props.allowEmptyWithDefault.default
      : optionValue;
  dropdownVisible.value = false;
}

function onToggleDropdown() {
  if (props.disabled) {
    return;
  }
  dropdownVisible.value = !dropdownVisible.value;
}
const reference = useTemplateRef<HTMLElement | null>('reference');
</script>
<template>
  <div
    ref="reference"
    class="p-select"
    :class="{
      'p-select-open': dropdownVisible,
      'p-select-disabled': props.disabled,
      'p-select-has-label': !!label,
      'p-select-has-icon-prepend': !!$slots['icon-prepend'],
    }"
    @click="onToggleDropdown"
  >
    <div class="p-select-inner">
      <p v-if="label" class="p-select-label">{{ label }}</p>
      <p
        v-if="optionLabel !== null"
        class="p-select-optionlabel"
        :class="{ 'p-select-optionlabel-empty': optionLabel === null }"
      >
        {{ optionLabel }}
      </p>
      <p v-else class="p-select-optionlabel p-select-optionlabel-empty">{{ placeholder }}</p>
    </div>
    <div v-if="$slots['icon-prepend']" class="p-select-icon-prepend">
      <slot name="icon-prepend" />
    </div>
    <div class="p-select-icon-append">
      <Icon class="p-select-icon" name="chevron-down" />
    </div>
  </div>
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
<style lang="scss" scoped>
$states: (
  'normal': '.p-select',
  'hovered': '.p-select:hover',
  'focused': '.p-select.p-select-open',
  'disabled': '.p-select.p-select-disabled',
);
@each $state, $state-selector in $states {
  #{$state-selector} {
    background-color: var(--p-select-#{$state}-background, var(--p-select-normal-background));
    border-radius: var(--p-select-#{$state}-border-radius, var(--p-select-normal-border-radius));
    color: var(--p-select-#{$state}-text-empty, var(--p-select-normal-text-empty));
    width: var(--p-select-#{$state}-width, var(--p-select-normal-width));
    height: var(--p-select-#{$state}-height, var(--p-select-normal-height));
    gap: var(--p-select-#{$state}-gap, var(--p-select-normal-gap));
    outline: var(--p-select-#{$state}-outline, var(--p-select-normal-outline));

    .p-select-inner {
      color: var(--p-select-#{$state}-text-filled, var(--p-select-normal-text-filled));
      padding-left: var(--p-select-#{$state}-padding-left, var(--p-select-normal-padding-left));
      padding-right: var(--p-select-#{$state}-height, var(--p-select-normal-height));
      padding-top: var(--p-select-#{$state}-padding-top, var(--p-select-normal-padding-top));
      padding-bottom: var(
        --p-select-#{$state}-padding-bottom,
        var(--p-select-normal-padding-bottom)
      );
    }
    .p-select-label {
      left: var(--p-select-#{$state}-padding-left, var(--p-select-normal-padding-left));
      top: var(--p-select-#{$state}-label-top, var(--p-select-normal-label-top));
      color: var(--p-select-#{$state}-label-color, var(--p-select-normal-label-color));
    }
    .p-select-icon-prepend,
    .p-select-icon-append {
      width: var(--p-select-#{$state}-height, var(--p-select-normal-height));
      height: var(--p-select-#{$state}-height, var(--p-select-normal-height));
      padding: var(--p-select-#{$state}-icon-padding, var(--p-select-normal-icon-padding));
    }
    .p-select-icon {
      color: var(--p-select-#{$state}-icon-color, var(--p-select-normal-icon-color));
    }

    .p-select-optionlabel {
      color: var(--p-select-#{$state}-text-filled, var(--p-select-normal-text-filled));
    }
    .p-select-optionlabel.p-select-optionlabel-empty {
      color: var(--p-select-#{$state}-text-empty, var(--p-select-normal-text-empty));
    }
  }

  #{$state-selector}.p-select-has-icon-prepend {
    .p-select-inner {
      padding-left: var(--p-select-#{$state}-height, var(--p-select-normal-height));
    }
    .p-select-label {
      left: var(--p-select-#{$state}-height, var(--p-select-normal-height));
    }
  }

  #{$state-selector}.p-select-has-label {
    .p-select-inner {
      padding-top: var(
        --p-select-#{$state}-padding-with-label-top,
        var(--p-select-normal-padding-with-label-top)
      );
      padding-bottom: var(
        --p-select-#{$state}-padding-with-label-bottom,
        var(--p-select-normal-padding-with-label-bottom)
      );
    }
  }
}

.p-select {
  display: flex;
  align-items: center;
  transition:
    outline 0.15s ease-in-out,
    background-color 0.15s ease-in-out,
    color 0.15s ease-in-out;
  cursor: pointer;
  position: relative;
}
.p-select-inner {
  display: flex;
  flex-direction: column;
  width: 100%;
}
.p-select-dropdown {
  display: flex;
  max-height: 16rem;
  overflow-y: auto;
  background-color: var(--p-select-dropdown-background);
  border-radius: var(--p-select-dropdown-border-radius);
  box-shadow: var(--p-select-dropdown-box-shadow);
  padding: var(--p-select-dropdown-padding);
}
.p-select-dropdown-inner {
  height: 100%;
  display: flex;
  flex-direction: column;
}
.p-select-optionlabel {
  text-overflow: ellipsis;
  overflow: hidden;
  white-space: nowrap;
  flex-grow: 1;
}
.p-select-label {
  font-size: 0.625rem;
  line-height: 1;
  transition: color 0.15s ease-in-out;
  position: absolute;
}
.p-select-icon-prepend {
  display: flex;
  align-items: center;
  justify-content: center;
  position: absolute;
  left: 0;
}
.p-select-icon-append {
  display: flex;
  align-items: center;
  justify-content: center;
  position: absolute;
  right: 0;
}
</style>
