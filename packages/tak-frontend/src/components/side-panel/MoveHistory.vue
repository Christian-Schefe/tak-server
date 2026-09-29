<script setup lang="ts">
import type { TakBaseGame } from '@/tak-core';
import { actionToString, gameResultToString } from '@/tak-core/ptn';
import { Button, ButtonGroup } from '@tak-ui-lib/components';
import { computed, onBeforeUnmount, onMounted } from 'vue';
import { LuChevronLeft, LuChevronRight, LuChevronsLeft, LuChevronsRight } from 'vue-icons-plus/lu';
import CollapseCard from './CollapseCard.vue';

type HistoryEntry =
  | {
      type: 'moveNumber' | 'gameResult';
      text: string;
    }
  | {
      type: 'whiteMove' | 'blackMove';
      text: string;
      plyIndex: number;
      active: boolean;
    };

const props = defineProps<{
  game: TakBaseGame;
  plyIndex: number | null;
}>();

const emit = defineEmits<{
  (e: 'updatePlyIndex', plyIndex: number | null): void;
}>();

const historyItems = computed(() => {
  const gamePlyIndex = props.plyIndex ?? props.game.actionHistory.length;
  const history = props.game.actionHistory;
  const items: HistoryEntry[][] = [];
  for (let i = 0; i < history.length; i += 2) {
    const row: HistoryEntry[] = [];
    const whiteMove = history[i];
    if (whiteMove === undefined) {
      continue;
    }
    const blackMove = i + 1 < history.length ? history[i + 1] : undefined;
    row.push({ type: 'moveNumber', text: `${(i / 2 + 1).toString()}.` });
    const whitePlyIndex = i + 1 === gamePlyIndex ? i : i + 1;
    row.push({
      type: 'whiteMove',
      text: actionToString(whiteMove.action),
      plyIndex: whitePlyIndex,
      active: whitePlyIndex < gamePlyIndex,
    });
    if (blackMove !== undefined) {
      const blackPlyIndex = i + 2 === gamePlyIndex ? i + 1 : i + 2;
      row.push({
        type: 'blackMove',
        text: actionToString(blackMove.action),
        plyIndex: blackPlyIndex,
        active: blackPlyIndex < gamePlyIndex,
      });
    }
    items.push(row);
  }
  if (props.game.gameResult !== null) {
    const row: HistoryEntry[] = [];
    row.push({ type: 'moveNumber', text: '' });
    row.push({ type: 'gameResult', text: gameResultToString(props.game.gameResult) });
    items.push(row);
  }
  return items;
});

function onClickMove(plyIndex: number | null) {
  const clampedPlyIndex = plyIndex !== null ? Math.max(0, plyIndex) : null;
  const newPlyIndex =
    clampedPlyIndex !== null && clampedPlyIndex >= props.game.actionHistory.length
      ? null
      : clampedPlyIndex;
  if (newPlyIndex === props.plyIndex) {
    return; // No change
  }
  emit('updatePlyIndex', newPlyIndex);
}

function onKeyDown(event: KeyboardEvent) {
  if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
    return;
  }
  if (event.key === 'ArrowLeft') {
    onClickMove((props.plyIndex ?? props.game.actionHistory.length) - 1);
  } else if (event.key === 'ArrowRight') {
    onClickMove((props.plyIndex ?? props.game.actionHistory.length) + 1);
  }
}
onMounted(() => {
  window.addEventListener('keydown', onKeyDown);
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeyDown);
});
</script>
<template>
  <CollapseCard title="Moves" grow>
    <div class="h-0 grow overflow-y-auto">
      <div
        v-for="(row, rowIndex) in historyItems"
        :key="rowIndex"
        class="flex px-2 py-0.5 rounded-md"
      >
        <template v-for="(item, itemIndex) in row" :key="itemIndex">
          <div v-if="item.type === 'moveNumber'" class="font-mono opacity-50 min-w-12">
            {{ item.text }}
          </div>
          <button
            v-if="item.type === 'whiteMove'"
            :class="`font-mono font-bold min-w-12 text-left hover:underline ${item.active ? '' : 'opacity-50'}`"
            @click="onClickMove(item.plyIndex)"
          >
            {{ item.text }}
          </button>
          <button
            v-if="item.type === 'blackMove'"
            :class="`font-mono font-bold min-w-12 text-left ml-4 hover:underline ${item.active ? '' : 'opacity-50'}`"
            @click="onClickMove(item.plyIndex)"
          >
            {{ item.text }}
          </button>
          <div v-if="item.type === 'gameResult'" class="font-mono font-bold">
            {{ item.text }}
          </div>
        </template>
      </div>
    </div>
    <div class="flex justify-center gap-2">
      <ButtonGroup>
        <Button
          size="small"
          icon-only
          severity="secondary"
          :disabled="props.plyIndex === 0"
          @click="onClickMove(0)"
        >
          <LuChevronsLeft />
        </Button>
        <Button
          size="small"
          icon-only
          severity="secondary"
          :disabled="props.plyIndex === 0"
          @click="onClickMove((plyIndex ?? game.actionHistory.length) - 1)"
        >
          <LuChevronLeft />
        </Button>
        <Button
          size="small"
          icon-only
          severity="secondary"
          :disabled="props.plyIndex === null"
          @click="onClickMove((plyIndex ?? game.actionHistory.length) + 1)"
        >
          <LuChevronRight />
        </Button>
        <Button
          size="small"
          icon-only
          severity="secondary"
          :disabled="props.plyIndex === null"
          @click="onClickMove(null)"
        >
          <LuChevronsRight />
        </Button>
      </ButtonGroup>
    </div>
  </CollapseCard>
</template>
