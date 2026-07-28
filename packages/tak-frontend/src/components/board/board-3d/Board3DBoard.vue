<script setup lang="ts">
import type { BoardPreset } from '@/features/board3dResources';
import { Vector3 } from 'three';
import type { GLTF } from 'three-stdlib';
import { computed } from 'vue';

const props = defineProps<{
  boardPreset: BoardPreset | undefined;
  gltf: GLTF | null;
}>();

const position = computed(() => {
  return new Vector3(...(props.boardPreset?.offset ?? []));
});
</script>
<template>
  <TresGroup :position="position">
    <primitive v-if="gltf" :object="gltf.scene"></primitive>
  </TresGroup>
</template>
