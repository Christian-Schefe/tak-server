<script setup lang="ts">
withDefaults(
  defineProps<{
    supportText?: string | undefined;
    label?: string | undefined;
    labelFor?: string | undefined;
    disabled?: boolean;
  }>(),
  {
    supportText: undefined,
    label: undefined,
    labelFor: undefined,
    disabled: false,
  },
);
</script>
<template>
  <div
    class="p-labelfield"
    :class="{
      'p-labelfield-disabled': disabled,
      'p-labelfield-has-label': !!label,
      'p-labelfield-has-icon-prepend': !!$slots['icon-prepend'],
      'p-labelfield-has-icon-append': !!$slots['icon-append'],
    }"
  >
    <div class="p-labelfield-inner">
      <div class="p-labelfield-slot">
        <slot />
      </div>
      <label v-if="label" class="p-labelfield-label" :for="labelFor">{{ label }}</label>
      <div v-if="$slots['icon-prepend']" class="p-labelfield-icon-prepend">
        <slot name="icon-prepend" />
      </div>
      <div v-if="$slots['icon-append']" class="p-labelfield-icon-append">
        <slot name="icon-append" />
      </div>
    </div>
    <p v-if="supportText" class="p-labelfield-support-text">
      {{ supportText }}
    </p>
  </div>
</template>
