<script setup lang="ts">
import type { TakBaseGame } from '@/tak-core/base.ts';
import type { TakAction, TakBaseGameSettings, TakGame, TakPlayer } from '@/tak-core/index.ts';
import Button from 'primevue/button';
import Dialog from 'primevue/dialog';
import { ref } from 'vue';
import { LuComputer, LuInfo, LuMessageCircle, LuSettings } from 'vue-icons-plus/lu';
import GameSettingsConfigure from './GameSettingsConfigure.vue';
import ChatPanel from './ChatPanel.vue';
import GameAnalysis from './GameAnalysis.vue';
import GameClock from './GameClock.vue';
import MoveHistory from './MoveHistory.vue';
import type { IconType } from 'vue-icons-plus/lib';
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

const sectionNames: Record<SidePanelSectionType, string> = {
  configure: 'Configure',
  analysis: 'Analysis',
  game_info: 'Game Info',
  full_game_info: 'Game Info',
  chat: 'Chat',
};

const openDialog = ref<SidePanelSection | null>(null);
const visible = ref(false);

const growingSection: Record<SidePanelSectionType, boolean> = {
  configure: false,
  analysis: false,
  game_info: true,
  full_game_info: true,
  chat: true,
};

const icons: Record<SidePanelSectionType, IconType> = {
  configure: LuSettings,
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
  <Dialog
    v-model:visible="visible"
    dismissable-mask
    :header="openDialog ? sectionNames[openDialog.type] : ''"
    :draggable="false"
    modal
    :style="{
      width: '90vw',
      maxWidth: '600px',
      height: openDialog && growingSection[openDialog.type] ? '90vh' : undefined,
      maxHeight: '800px',
    }"
  >
    <GameSettingsConfigure
      v-if="openDialog?.type === 'configure'"
      @apply="$emit('settingsSubmit', $event)"
    ></GameSettingsConfigure>
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
