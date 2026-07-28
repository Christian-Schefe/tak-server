import type { TakBaseGame } from '@/tak-core';
import type { MaybeRefOrGetter } from 'vue';
import { useSound } from '@vueuse/sound';
import { computed, watch } from 'vue';
import { toValue } from 'vue';
import { useSettingsStore } from './settings';

export function usePlayGameActionSound(game: MaybeRefOrGetter<TakBaseGame | undefined>) {
  const settingsStore = useSettingsStore();

  const { play } = useSound('/sounds/basic/action.ogg', {
    volume: computed(() => settingsStore.settings.audioVolume),
    autoplay: false,
  });

  watch(
    () => toValue(game),
    (newGame, oldGame) => {
      if (settingsStore.settings.audioVolume === 0) {
        return;
      }
      if (newGame && oldGame && newGame.actionHistory.length !== oldGame.actionHistory.length) {
        console.log('Action history changed, playing sound');
        play();
      }
    },
  );
}
