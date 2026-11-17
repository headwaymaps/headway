import type { IControl } from 'maplibre-gl';
import { watch, type WatchStopHandle } from 'vue';
import { debugEnabled } from 'src/utils/debug';

/// A checkbox turning debug tooling on, which the debug panel's own checkbox replaces once it is.
export default class DebugControl implements IControl {
  containerEl: HTMLElement;
  stopWatching?: WatchStopHandle;

  constructor() {
    this.containerEl = document.createElement('label');
    this.containerEl.className =
      'headway-ctrl-debug maplibregl-ctrl maplibregl-ctrl-group';
  }

  onAdd(): HTMLElement {
    const checkbox = document.createElement('input');
    checkbox.type = 'checkbox';
    checkbox.addEventListener('change', () => {
      debugEnabled.value = checkbox.checked;
    });
    this.stopWatching = watch(
      debugEnabled,
      (enabled) => {
        checkbox.checked = enabled;
        this.containerEl.style.display = enabled ? 'none' : '';
      },
      { immediate: true },
    );
    this.containerEl.replaceChildren(checkbox, 'Debug');
    return this.containerEl;
  }

  onRemove(): void {
    this.stopWatching?.();
    this.containerEl.remove();
  }
}
