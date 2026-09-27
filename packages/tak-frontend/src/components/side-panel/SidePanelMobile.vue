<script setup lang="ts">
import type { SidePanelSection, SidePanelSectionType } from '@/features/sidePanel.ts';
import type { TakBaseGame } from '@/tak-core/base.ts';
import type { TakAction, TakBaseGameSettings, TakGame, TakPlayer } from '@/tak-core/index.ts';
import { Button, Dialog } from '@tak-ui-lib/components';
import { ref } from 'vue';
import type { IconType } from 'vue-icons-plus/lib';
import { LuComputer, LuInfo, LuMessageCircle } from 'vue-icons-plus/lu';
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

const sectionNames: Record<SidePanelSectionType, string> = {
  analysis: 'Analysis',
  game_info: 'Game Info',
  full_game_info: 'Game Info',
  chat: 'Chat',
};

const openDialog = ref<SidePanelSection | null>(null);
const visible = ref(false);

const icons: Record<SidePanelSectionType, IconType> = {
  analysis: LuComputer,
  game_info: LuInfo,
  full_game_info: LuInfo,
  chat: LuMessageCircle,
};
</script>
<template>
  <div class="w-full h-full flex items-stretch">
    <Button
      v-for="section in sections"
      :key="section.type"
      variant="text"
      severity="secondary"
      class="w-0! grow"
      @click="
        openDialog = section;
        visible = true;
      "
    >
      <template #icon>
        <component :is="icons[section.type]" class="w-6 h-6"></component>
      </template>
    </Button>
  </div>
  <Dialog v-model:visible="visible" :header="openDialog ? sectionNames[openDialog.type] : ''">
    <GameAnalysis
      v-if="openDialog?.type === 'analysis'"
      :game="game"
      :ply-index="plyIndex"
      @action="$emit('analysisAction', $event)"
    ></GameAnalysis>
    <MoveHistory
      v-if="openDialog?.type === 'game_info'"
      :game="game"
      :ply-index="plyIndex"
      @update-ply-index="plyIndex = $event"
    ></MoveHistory>
    <ChatPanel
      v-if="openDialog?.type === 'chat' && openDialog.conversation"
      :conversation="openDialog.conversation"
    />
    <div v-if="openDialog?.type === 'full_game_info'" class="flex flex-col h-full">
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
  </Dialog>
</template>
