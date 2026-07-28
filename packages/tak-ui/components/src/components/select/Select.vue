<script setup lang="ts" generic="T">
import {
  autoUpdate,
  flip,
  limitShift,
  offset,
  shift,
  useFloating,
  type Placement,
} from '@floating-ui/vue';
import { computed, onMounted, onUnmounted, ref, useTemplateRef } from 'vue';
import { useOverlayZIndex } from '../../overlay';
import { Button } from '../button';
import { Icon } from '../icon';

const value = defineModel<T>({ required: true });
const props = withDefaults(
  defineProps<{
    options: { value: T; label: string }[];
    allowEmptyWithDefault?: { default: T } | undefined;
    placeholder?: string;
    label?: string | undefined;
    placement?: Placement;
    cmp?: (a: T, b: T) => boolean;
  }>(),
  {
    allowEmptyWithDefault: undefined,
    placeholder: 'Placeholder',
    placement: 'bottom-start',
    label: undefined,
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

const reference = useTemplateRef<HTMLElement | null>('reference');
const floating = useTemplateRef<HTMLElement | null>('floating');
const { floatingStyles } = useFloating(reference, floating, {
  placement: () => props.placement,
  middleware: [offset(10), flip({ padding: 5 }), shift({ limiter: limitShift(), padding: 5 })],
  whileElementsMounted: autoUpdate,
});

function onPointerDownOutside(event: PointerEvent) {
  if (
    reference.value?.contains(event.target as Node) !== true &&
    floating.value?.contains(event.target as Node) !== true
  ) {
    dropdownVisible.value = false;
  }
}

function onSelectOption(optionValue: T) {
  value.value =
    value.value === optionValue && props.allowEmptyWithDefault !== undefined
      ? props.allowEmptyWithDefault.default
      : optionValue;
  dropdownVisible.value = false;
}

const zIndex = useOverlayZIndex(floating, dropdownVisible, 1);

onMounted(() => {
  document.addEventListener('pointerdown', onPointerDownOutside);
});
onUnmounted(() => {
  document.removeEventListener('pointerdown', onPointerDownOutside);
});
</script>
<template>
  <div
    ref="reference"
    class="p-select"
    :class="{ 'p-select-open': dropdownVisible, 'p-select-with-label': !!label }"
    @click="dropdownVisible = !dropdownVisible"
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
    <Icon class="p-select-icon" name="chevron-down" />
  </div>
  <Teleport to="body">
    <Transition>
      <div v-if="dropdownVisible" ref="floating" :style="{ ...floatingStyles, zIndex }">
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
      </div>
    </Transition>
  </Teleport>
</template>
<style lang="css" scoped>
.v-enter-active,
.v-leave-active {
  transition: opacity 0.1s ease;
}
.v-enter-active .p-select-dropdown,
.v-leave-active .p-select-dropdown {
  transition: transform 0.1s ease;
}

.v-enter-from,
.v-leave-to {
  opacity: 0;
}

.v-enter-from .p-select-dropdown,
.v-leave-to .p-select-dropdown {
  transform: scale(0.9);
}

.p-select {
  display: flex;
  align-items: center;
  color: var(--p-select-empty-text);
  background-color: var(--p-select-background);
  padding-left: var(--p-select-padding-x);
  padding-right: var(--p-select-padding-x);
  gap: var(--p-select-padding-x);
  border: var(--p-select-border);
  border-radius: var(--p-select-border-radius);
  transition: border 0.2s ease-in-out;
  cursor: pointer;
  position: relative;
}
.p-select-inner {
  display: flex;
  flex-direction: column;
  flex-grow: 1;
  padding-top: var(--p-inputtext-padding-y);
  padding-bottom: var(--p-inputtext-padding-y);
}
.p-select:hover {
  border: var(--p-select-border-hover);
}
.p-select.p-select-open {
  border: var(--p-select-border-focus);
}
.p-select.p-select-with-label .p-select-inner {
  padding-top: calc(var(--p-select-padding-y) + 0.625rem);
}
.p-select-dropdown {
  display: flex;
  background-color: var(--p-select-dropdown-background);
  padding: var(--p-select-dropdown-padding);
  border-radius: var(--p-select-dropdown-border-radius);
  box-shadow: var(--p-select-dropdown-box-shadow);
  max-height: 16rem;
  overflow-y: auto;
}
.p-select-dropdown-inner {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: var(--p-select-dropdown-gap);
}
.p-select-optionlabel {
  color: var(--p-select-filled-text);
  flex-grow: 1;
}
.p-select-optionlabel-empty {
  color: var(--p-select-empty-text);
}
.p-select-icon {
  color: var(--p-select-icon-color);
}
.p-select-label {
  font-size: 0.625rem;
  line-height: 1;
  transition: color 0.2s ease-in-out;
  position: absolute;
  top: var(--p-select-padding-y);
  left: var(--p-select-padding-x);
}
.p-select.p-select-open .p-select-label {
  color: var(--p-select-label-text-focus);
}
</style>
