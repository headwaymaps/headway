import { ref, watch } from 'vue';

const STORAGE_KEY = 'debug.enabled';

/// Whether developer tooling like the map HUD is on. `?debug` turns it on and `?debug=0` off, and
/// either is remembered, so it stays as set across pages and reloads.
export const debugEnabled = ref(initiallyEnabled());

function initiallyEnabled(): boolean {
  const param = new URLSearchParams(window.location.search).get('debug');
  if (param !== null) return param !== '0';
  return window.localStorage.getItem(STORAGE_KEY) === 'true';
}

watch(
  debugEnabled,
  (enabled) => {
    window.localStorage.setItem(STORAGE_KEY, String(enabled));
    // Otherwise the page's own `?debug` would turn it back on at the next reload.
    const url = new URL(window.location.href);
    if (!enabled && url.searchParams.has('debug')) {
      url.searchParams.delete('debug');
      window.history.replaceState(window.history.state, '', url);
    }
  },
  { immediate: true },
);
