import Config from 'src/utils/Config';

/// Whether developer tooling like the map HUD is on: by config, or by opening the app with `?debug`.
///
/// The URL is read once, at load, so it survives in-app navigation without every route having to
/// carry it.
const debugInUrl = new URLSearchParams(window.location.search).has('debug');

export function debugEnabled(): boolean {
  return debugInUrl || Config.debugEnabled;
}
