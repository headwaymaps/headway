export function supportsHover(): boolean {
  return window.matchMedia('(hover: hover)').matches;
}
