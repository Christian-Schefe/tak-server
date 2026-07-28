<script setup lang="ts">
import { checkEngineSettings, evaluatePosition, initializeEngine, stopEngine } from '@/api/engine';
import { type TakAction, type TakBaseGame } from '@/tak-core';
import { actionFromString } from '@/tak-core/ptn';
import Button from 'primevue/button';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';

export interface EvalVariation {
  evaluation: number;
  moves: string[];
}
const engineKey = 'analysis-worker';
const props = defineProps<{
  game: TakBaseGame;
  plyIndex: number | null;
}>();

const emit = defineEmits<{
  (e: 'action', action: TakAction): void;
}>();

const hasLoaded = ref(false);

const variations = ref<EvalVariation[]>([]);
const evaluationSupported = ref<null | boolean>(null);
const currentEvaluationKey = ref<string | null>(null);

onMounted(() => {
  void onInit();
});

onUnmounted(() => {
  void stopEngine(engineKey);
});

async function onInit() {
  await initializeEngine(engineKey, (message) => {
    if (message.type === 'evaluation') {
      if (message.key !== currentEvaluationKey.value) {
        console.warn('Received evaluation for old position, ignoring.');
        return;
      }
      variations.value = message.variations;
    } else {
      evaluationSupported.value = message.supported;
    }
  });
  hasLoaded.value = true;
}

const shownGame = computed(() => {
  const game = props.game;
  const plyIndex = props.plyIndex;
  if (plyIndex === null) {
    return game;
  }
  const gameClone = game.clone();
  gameClone.trimToPlyIndex(plyIndex);
  return gameClone;
});

const isEvaluationSupported = computed(() => {
  return evaluationSupported.value === true && shownGame.value.isOngoing();
});

watch([hasLoaded, () => shownGame.value.settings], ([newHasLoaded, newSettings]) => {
  if (!newHasLoaded) {
    return;
  }
  evaluationSupported.value = null;
  console.log('Game settings changed, checking engine settings...');
  void checkEngineSettings(engineKey, newSettings);
});

watch(
  [hasLoaded, shownGame, evaluationSupported],
  ([newhasLoaded, newShownGame, newEvaluationSupported]) => {
    if (!newhasLoaded) {
      return;
    }
    const game = newShownGame;
    if (!game.isOngoing()) {
      void stopEngine(engineKey);
      console.log('Game is not ongoing, stopping engine.');
      return;
    }
    if (newEvaluationSupported === null) {
      console.log('Evaluation support not yet determined, waiting...');
      return;
    }
    if (!newEvaluationSupported) {
      void stopEngine(engineKey);
      console.log('Evaluation not supported or disabled, stopping engine.');
    } else {
      const key = crypto.randomUUID();
      currentEvaluationKey.value = key;
      void evaluatePosition(engineKey, key, game);

      console.log('Evaluating position...');
    }
  },
);

const adjustedVariations = computed(() => {
  const variationData: (
    | (EvalVariation & {
        displayMoves: string;
        displayEvaluation: string;
      })
    | null
  )[] = ([] = variations.value.map((variation) => ({
    ...variation,
    displayMoves: variation.moves.slice(0, 12).join(' '),
    displayEvaluation:
      (variation.evaluation > 0 ? '+' : variation.evaluation < 0 ? '-' : '') +
      (Math.abs(variation.evaluation) / 10).toFixed(1),
  })));
  while (variationData.length < 3) {
    variationData.push(null);
  }
  return variationData;
});

function onClickVariation(variation: EvalVariation) {
  const move = variation.moves[0];
  if (move === undefined) return;
  const action = actionFromString(move);
  if (!action) {
    console.error('Invalid move in variation:', move);
    return;
  }
  emit('action', action);
}
</script>
<template>
  <div
    v-if="isEvaluationSupported"
    class="w-full flex flex-col mt-1 p-0.5 gap-1 bg-surface-100 dark:bg-surface-800 rounded-sm border border-surface"
  >
    <div
      v-for="(item, index) in adjustedVariations"
      :key="index"
      class="flex flex-row items-center gap-2"
    >
      <Button
        v-if="item"
        severity="secondary"
        class="w-full! h-6.5! p-0! text-left! cursor-pointer flex items-center gap-2"
        @click="onClickVariation(item)"
      >
        <span
          :class="`px-1 py-0.5 font-bold rounded-sm font-mono text-sm border border-surface ${item.evaluation >= 0 ? 'bg-surface-0 text-surface-800' : 'bg-surface-950 text-surface-100'}`"
        >
          {{ item.displayEvaluation }}
        </span>
        <span
          class="w-0 grow font-mono text-sm text-nowrap overflow-hidden text-ellipsis text-color"
        >
          {{ item.displayMoves }}
        </span>
      </Button>
      <div v-else class="h-6.5"></div>
    </div>
  </div>
  <p v-else class="text-muted-color">No analysis available.</p>
</template>
