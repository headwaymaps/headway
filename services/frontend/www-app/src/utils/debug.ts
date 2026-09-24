/// Whether the app was opened with `?debug`, which turns on developer tooling like the map HUD.
///
/// Read once, at load, so it survives in-app navigation without every route having to carry it.
export const debugEnabled = new URLSearchParams(window.location.search).has(
  'debug',
);
