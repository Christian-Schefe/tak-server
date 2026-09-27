<script setup lang="ts">
import type { SidePanelSection, SidePanelSectionType } from '@/features/sidePanel.ts';
import type { TakBaseGame } from '@/tak-core/base.ts';
import type { TakAction, TakBaseGameSettings, TakGame, TakPlayer } from '@/tak-core/index.ts';
import { ref } from 'vue';
import ChatPanel from './ChatPanel.vue';
import GameAnalysis from './GameAnalysis.vue';
import GameClock from './GameClock.vue';
import MoveHistory from './MoveHistory.vue';

defineProps<{
  game: TakBaseGame;
  fullGame?: TakGame;
  playerIds?: Record<TakPlayer, string>;
  sections: SidePanelSection[];
}>();

const plyIndex = defineModel<number | null>('plyIndex', { required: true });

defineEmits<{
  settingsSubmit: [TakBaseGameSettings];
  analysisAction: [TakAction];
}>();

const activeAccordionIndex = ref<string | string[] | null | undefined>([
  'game_info',
  'full_game_info',
  'chat',
]);

const sectionNames: Record<SidePanelSectionType, string> = {
  analysis: 'Analysis',
  game_info: 'Game Info',
  full_game_info: 'Game Info',
  chat: 'Chat',
};

const growingSection: Record<SidePanelSectionType, boolean> = {
  analysis: false,
  game_info: true,
  full_game_info: true,
  chat: true,
};
</script>
<template>
  <div class="flex flex-col grow">
    <div
      v-for="section in sections"
      :key="section.type"
      :class="`flex flex-col ${activeAccordionIndex?.includes(section.type) && growingSection[section.type] ? 'grow' : ''}`"
      :style="{ transition: 'flex-grow 200ms ease-in-out' }"
    >
      <h2>{{ sectionNames[section.type] }}</h2>
      <GameAnalysis
        v-if="section.type === 'analysis' && activeAccordionIndex?.includes('analysis')"
        :game="game"
        :ply-index="plyIndex"
        @action="$emit('analysisAction', $event)"
      ></GameAnalysis>
      <MoveHistory
        v-if="section.type === 'game_info'"
        :game="game"
        :ply-index="plyIndex"
        @update-ply-index="plyIndex = $event"
      ></MoveHistory>
      <ChatPanel
        v-if="section.type === 'chat' && section.conversation"
        :conversation="section.conversation"
      />
      <div v-if="section.type === 'full_game_info'" class="flex flex-col grow">
        <GameClock
          v-if="fullGame && playerIds"
          :game="fullGame"
          player="white"
          :player-id="playerIds.white"
        ></GameClock>
        <GameClock
          v-if="fullGame && playerIds"
          :game="fullGame"
          player="black"
          :player-id="playerIds.black"
        ></GameClock>
        <MoveHistory
          :game="game"
          :ply-index="plyIndex"
          @update-ply-index="plyIndex = $event"
        ></MoveHistory>
      </div>
    </div>
  </div>
</template>
