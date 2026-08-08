<script setup lang="ts">
import { useFormValue } from '../../form';

const model = defineModel<string>({ default: '' });
const props = withDefaults(
  defineProps<{
    name?: string | undefined;
    placeholder?: string | undefined;
    supportText?: string | undefined | false;
    label?: string | undefined;
    inputId?: string | undefined;
    disabled?: boolean;
  }>(),
  {
    name: undefined,
    placeholder: undefined,
    supportText: undefined,
    label: undefined,
    inputId: undefined,
    disabled: false,
  },
);
const emit = defineEmits<{
  change: [string];
}>();

function onInputChange(event: Event) {
  const target = event.target as HTMLInputElement;
  model.value = target.value;
  emit('change', target.value);
}

useFormValue(model, () => props.name);
</script>
<template>
  <div class="p-inputtext-wrapper" :class="{ 'p-inputtext-disabled': disabled }">
    <div class="p-inputtext-container">
      <div class="p-inputtext-container-inner">
        <input
          :id="inputId"
          v-model="model"
          :name="name"
          class="p-inputtext"
          :placeholder="placeholder"
          :disabled="disabled"
          @change="onInputChange"
        />
        <label v-if="label" class="p-inputtext-label" :for="inputId">{{ label }}</label>
      </div>
      <slot />
    </div>
    <p v-if="supportText !== false" class="p-inputtext-support-text">{{ supportText ?? '&nbsp;' }}</p>
  </div>
</template>
<style lang="scss">
$states: (
  'normal': '.p-inputtext-wrapper',
  'hovered': '.p-inputtext-wrapper:hover',
  'focused': '.p-inputtext-wrapper:focus-within',
  'disabled': '.p-inputtext-wrapper.p-inputtext-disabled',
);
@each $state, $state-selector in $states {
  #{$state-selector} {
    .p-inputtext-container {
      background-color: var(
        --p-inputtext-#{$state}-background,
        var(--p-inputtext-normal-background)
      );
      border-radius: var(
        --p-inputtext-#{$state}-border-radius,
        var(--p-inputtext-normal-border-radius)
      );
      color: var(--p-inputtext-#{$state}-text-empty, var(--p-inputtext-normal-text-empty));
      width: var(--p-inputtext-#{$state}-width, var(--p-inputtext-normal-width));
      padding-right: var(--p-inputtext-#{$state}-padding-x, var(--p-inputtext-normal-padding-x));
      gap: var(--p-inputtext-#{$state}-gap, var(--p-inputtext-normal-gap));
      outline: var(--p-inputtext-#{$state}-outline, var(--p-inputtext-normal-outline));
    }
    .p-inputtext {
      color: var(--p-inputtext-#{$state}-text-filled, var(--p-inputtext-normal-text-filled));
      padding-left: var(--p-inputtext-#{$state}-padding-x, var(--p-inputtext-normal-padding-x));
      padding-top: var(--p-inputtext-#{$state}-padding-y, var(--p-inputtext-normal-padding-y));
      padding-bottom: var(--p-inputtext-#{$state}-padding-y, var(--p-inputtext-normal-padding-y));
    }
    .p-inputtext::placeholder {
      color: var(--p-inputtext-#{$state}-text-empty, var(--p-inputtext-normal-text-empty));
    }
    .p-inputtext-label {
      left: calc(
        var(--p-inputtext-#{$state}-padding-x, var(--p-inputtext-normal-padding-x)) - var(
            --p-inputtext-#{$state}-label-padding,
            var(--p-inputtext-normal-label-padding)
          )
      );
      background-color: var(
        --p-inputtext-#{$state}-background,
        var(--p-inputtext-normal-background)
      );
      padding-left: var(
        --p-inputtext-#{$state}-label-padding,
        var(--p-inputtext-normal-label-padding)
      );
      padding-right: var(
        --p-inputtext-#{$state}-label-padding,
        var(--p-inputtext-normal-label-padding)
      );
      color: var(--p-inputtext-#{$state}-label-color, var(--p-inputtext-normal-label-color));
    }
    .p-inputtext-support-text {
      color: var(--p-inputtext-#{$state}-support-color, var(--p-inputtext-normal-support-color));
      padding: var(
        --p-inputtext-#{$state}-support-padding,
        var(--p-inputtext-normal-support-padding)
      );
    }
  }
}

.p-inputtext-wrapper {
  display: flex;
  flex-direction: column;
}
.p-inputtext-support-text {
  font-size: 0.75rem;
  line-height: 1;
}

.p-inputtext-container {
  margin: 0;
  display: flex;
  max-width: 100%;
  position: relative;
  align-items: center;
  transition:
    outline-color 0.2s ease,
    background-color 0.2s ease;
}
.p-inputtext-container-inner {
  display: flex;
  flex-direction: column;
  flex-grow: 1;
}
.p-inputtext {
  width: 100%;
  border: none;
  outline: none;
  margin: 0;
}
.p-inputtext-label {
  font-size: 0.625rem;
  line-height: 1;
  transition: color 0.2s ease-in-out;
  position: absolute;
  transform: translateY(-50%);
}
</style>
