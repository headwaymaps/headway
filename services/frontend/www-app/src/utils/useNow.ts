import { onMounted, onUnmounted, Ref, ref } from 'vue';

/// How often `useNow` ticks: often enough that a minute countdown is never far behind.
const TICK_MS = 30_000;

/// A Vue composable giving the calling component its own clock, in epoch milliseconds, that
/// re-renders it every `TICK_MS` while mounted.
export function useNow(): Ref<number> {
  const now = ref(Date.now());
  let clock: ReturnType<typeof setInterval> | undefined;
  onMounted(() => {
    clock = setInterval(() => (now.value = Date.now()), TICK_MS);
  });
  onUnmounted(() => clearInterval(clock));
  return now;
}
