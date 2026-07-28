<script setup lang="ts">
import { ref } from 'vue';
import GameSettingsConfigure from './GameSettingsConfigure.vue';
import ChatPanel from './ChatPanel.vue';
import GameAnalysis from './GameAnalysis.vue';
import MoveHistory from './MoveHistory.vue';
import type { TakBaseGame } from '@/tak-core/base.ts';
import type { TakAction, TakBaseGameSettings, TakGame, TakPlayer } from '@/tak-core/index.ts';
import Accordion from 'primevue/accordion';
import AccordionPanel from 'primevue/accordionpanel';
import AccordionHeader from 'primevue/accordionheader';
import AccordionContent from 'primevue/accordioncontent';
import GameClock from './GameClock.vue';
import type { SidePanelSection, SidePanelSectionType } from '@/features/sidePanel.ts';

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
  configure: 'Configure',
  analysis: 'Analysis',
  game_info: 'Game Info',
  full_game_info: 'Game Info',
  chat: 'Chat',
};

const growingSection: Record<SidePanelSectionType, boolean> = {
  configure: false,
  analysis: false,
  game_info: true,
  full_game_info: true,
  chat: true,
};
</script>
<template>
  <Accordion v-model:value="activeAccordionIndex" multiple class="h-full flex flex-col">
    <AccordionPanel
      v-for="section in sections"
      :key="section.type"
      :value="section.type"
      :class="
        activeAccordionIndex?.includes(section.type) && growingSection[section.type] ? 'grow' : ''
      "
      :style="{ transition: 'flex-grow 200ms ease-in-out' }"
    >
      <AccordionHeader>{{ sectionNames[section.type] }}</AccordionHeader>
      <AccordionContent
        :class="growingSection[section.type] ? 'h-0 grow' : ''"
        :pt="{
          contentWrapper: { style: { height: '100%' } },
          content: { style: { height: '100%', display: 'flex', flexDirection: 'column' } },
        }"
      >
        <GameSettingsConfigure
          v-if="section.type === 'configure'"
          @apply="$emit('settingsSubmit', $event)"
        ></GameSettingsConfigure>
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
        <div v-if="section.type === 'full_game_info'" class="flex flex-col h-full">
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
      </AccordionContent>
    </AccordionPanel>
  </Accordion>
</template>
