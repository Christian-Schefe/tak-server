<script setup lang="ts">
import type { TablePreset } from '@/features/board3dResources';
import { Euler, MathUtils, Vector3 } from 'three';
import { type GLTF } from 'three-stdlib';
import { computed } from 'vue';

const props = defineProps<{
  tablePreset: TablePreset | undefined;
  gltf: GLTF | null;
}>();

const position = computed(() => {
  return new Vector3(...(props.tablePreset?.offset ?? []));
});

const rotation = computed(() => {
  const radiansRotation = (props.tablePreset?.rotation ?? []).map((angle) =>
    MathUtils.degToRad(angle),
  );
  return new Euler(...radiansRotation);
});
</script>
<template>
  <TresGroup :position="position" :rotation="rotation">
    <primitive v-if="gltf" :object="gltf.scene"></primitive>
  </TresGroup>
</template>
