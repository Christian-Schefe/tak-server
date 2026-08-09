<script setup lang="ts">
import { useFormValue } from '../../form';

const model = defineModel<string>({ default: '' });
const props = withDefaults(
  defineProps<{
    name?: string | undefined;
    placeholder?: string | undefined;
    supportText?: string | undefined;
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
  <div
    class="p-inputtext-wrapper"
    :class="{
      'p-inputtext-disabled': disabled,
      'p-inputtext-has-icon-prepend': !!$slots['icon-prepend'],
      'p-inputtext-has-icon-append': !!$slots['icon-append'],
    }"
  >
    <div class="p-inputtext">
      <input
        :id="inputId"
        v-model="model"
        :name="name"
        class="p-inputtext-input"
        :placeholder="placeholder"
        :disabled="disabled"
        type="text"
        @change="onInputChange"
      />
      <label v-if="label" class="p-inputtext-label" :for="inputId">{{ label }}</label>
      <div v-if="$slots['icon-prepend']" class="p-inputtext-icon-prepend">
        <slot name="icon-prepend" />
      </div>
      <div v-if="$slots['icon-append']" class="p-inputtext-icon-append">
        <slot name="icon-append" />
      </div>
    </div>
    <p v-if="supportText" class="p-inputtext-support-text">
      {{ supportText }}
    </p>
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
    .p-inputtext {
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
      height: var(--p-inputtext-#{$state}-height, var(--p-inputtext-normal-height));
      gap: var(--p-inputtext-#{$state}-gap, var(--p-inputtext-normal-gap));
      outline: var(--p-inputtext-#{$state}-outline, var(--p-inputtext-normal-outline));
    }
    .p-inputtext-input {
      color: var(--p-inputtext-#{$state}-text-filled, var(--p-inputtext-normal-text-filled));
      padding-left: var(
        --p-inputtext-#{$state}-padding-left,
        var(--p-inputtext-normal-padding-left)
      );
      padding-right: var(
        --p-inputtext-#{$state}-padding-right,
        var(--p-inputtext-normal-padding-right)
      );
      padding-top: var(--p-inputtext-#{$state}-padding-top, var(--p-inputtext-normal-padding-top));
      padding-bottom: var(
        --p-inputtext-#{$state}-padding-bottom,
        var(--p-inputtext-normal-padding-bottom)
      );
    }
    .p-inputtext::placeholder {
      color: var(--p-inputtext-#{$state}-text-empty, var(--p-inputtext-normal-text-empty));
    }
    .p-inputtext-label {
      left: var(--p-inputtext-#{$state}-padding-left, var(--p-inputtext-normal-padding-left));
      top: var(--p-inputtext-#{$state}-label-top, var(--p-inputtext-normal-label-top));
      color: var(--p-inputtext-#{$state}-label-color, var(--p-inputtext-normal-label-color));
    }
    .p-inputtext-support-text {
      color: var(--p-inputtext-#{$state}-support-color, var(--p-inputtext-normal-support-color));
      padding: var(
        --p-inputtext-#{$state}-support-padding,
        var(--p-inputtext-normal-support-padding)
      );
    }
    .p-inputtext-icon-prepend,
    .p-inputtext-icon-append {
      width: var(--p-inputtext-#{$state}-height, var(--p-inputtext-normal-height));
      height: var(--p-inputtext-#{$state}-height, var(--p-inputtext-normal-height));
      padding: var(--p-inputtext-#{$state}-icon-padding, var(--p-inputtext-normal-icon-padding));
    }
  }

  #{$state-selector}.p-inputtext-has-icon-prepend {
    .p-inputtext-input {
      padding-left: var(--p-inputtext-#{$state}-height, var(--p-inputtext-normal-height));
    }
    .p-inputtext-label {
      left: var(--p-inputtext-#{$state}-height, var(--p-inputtext-normal-height));
    }
  }
  #{$state-selector}.p-inputtext-has-icon-append {
    .p-inputtext-input {
      padding-right: var(--p-inputtext-#{$state}-height, var(--p-inputtext-normal-height));
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

.p-inputtext {
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
.p-inputtext-input {
  width: 100%;
  height: 100%;
  border: none;
  outline: none;
  margin: 0;
}
.p-inputtext-label {
  font-size: 0.625rem;
  line-height: 1;
  transition: color 0.2s ease-in-out;
  position: absolute;
}
.p-inputtext-icon-prepend {
  display: flex;
  align-items: center;
  justify-content: center;
  position: absolute;
  left: 0;
}
.p-inputtext-icon-append {
  display: flex;
  align-items: center;
  justify-content: center;
  position: absolute;
  right: 0;
}
</style>
