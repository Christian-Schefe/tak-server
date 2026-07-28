import type { ChatMessageConversation } from '@/api/chat';

export type SidePanelSectionType = SidePanelSection['type'];

export type SidePanelSection =
  | {
      type: 'chat';
      conversation?: ChatMessageConversation;
    }
  | {
      type: 'configure';
    }
  | {
      type: 'analysis';
    }
  | {
      type: 'game_info';
    }
  | {
      type: 'full_game_info';
    };
