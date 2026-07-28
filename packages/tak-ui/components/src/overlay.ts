import {
  computed,
  inject,
  onUnmounted,
  ref,
  toValue,
  watch,
  type App,
  type InjectionKey,
  type MaybeRefOrGetter,
} from 'vue';

export class OverlayManager {
  overlays: (Overlay | undefined)[][] = [];

  addOverlay(overlay: Overlay, priority: number): number {
    const priorityOverlays = this.overlays[priority] ?? [];
    const index = priorityOverlays.findIndex((o) => o === undefined);
    const overlayIndex = index !== -1 ? index : priorityOverlays.length;

    priorityOverlays[overlayIndex] = overlay;
    this.overlays[priority] = priorityOverlays;
    return overlayIndex;
  }

  removeOverlay(index: number, priority: number): void {
    if (this.overlays[priority] === undefined) {
      return;
    }
    this.overlays[priority][index] = undefined;
  }
}

export type Overlay = HTMLElement;

const OverlayKey: InjectionKey<OverlayManager> = Symbol('OverlayManager');

export function provideOverlayManager(app: App) {
  const overlayManager = new OverlayManager();
  app.provide(OverlayKey, overlayManager);
}

export function useOverlayManager(): OverlayManager {
  const overlayManager = inject(OverlayKey);
  if (!overlayManager) {
    throw new Error('OverlayManager not found');
  }
  return overlayManager;
}

export function useOverlayZIndex(
  overlay: MaybeRefOrGetter<Overlay | undefined | null>,
  isVisible: MaybeRefOrGetter<boolean>,
  priority: number = 0,
) {
  const overlayManager = useOverlayManager();
  const index = ref<number | undefined>(undefined);
  watch(
    [() => toValue(overlay), () => toValue(isVisible)],
    ([newOverlay, newIsVisible]) => {
      if (index.value !== undefined) {
        overlayManager.removeOverlay(index.value, priority);
      }
      if (newOverlay && newIsVisible) {
        index.value = overlayManager.addOverlay(newOverlay, priority);
      } else {
        index.value = undefined;
      }
    },
    { immediate: true },
  );
  onUnmounted(() => {
    if (index.value !== undefined) {
      overlayManager.removeOverlay(index.value, priority);
    }
  });
  return computed(() =>
    index.value !== undefined ? index.value + 1000 + priority * 1000 : undefined,
  );
}
