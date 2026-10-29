<template>
  <div class="trip-corpus">
    <div class="trip-corpus-controls">
      <b>Trip corpus</b>
      <button type="button" @click="step(-1)">← prev (k)</button>
      <button type="button" @click="step(1)">next (j) →</button>
      <span class="k">{{ corpus.length }} trips</span>
    </div>
    <q-virtual-scroll
      class="trip-corpus-list"
      :items="corpus"
      :virtual-scroll-item-size="28"
    >
      <template #default="{ item, index }">
        <div
          :key="index"
          class="trip-corpus-row"
          :class="{ selected: index === selected }"
          @click="select(index)"
        >
          #{{ item.id }} {{ title(item) }}
        </div>
      </template>
    </q-virtual-scroll>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { LngLat } from 'maplibre-gl';

/// Served by the compare helper, through the dev server's `/otp-compare` proxy.
const CORPUS_URL = '/otp-compare/corpus.json';
/// Endpoints this close, in meters, are the corpus trip's.
const SAME_PLACE_METERS = 1;
const GENERIC_STREETS = new Set([
  'road',
  'path',
  'bike path',
  'sidewalk',
  'service road',
  'link',
  'steps',
  'alley',
  'parking aisle',
  'crosswalk',
  'track',
  'footbridge',
  'ramp',
]);

type LatLon = [number, number];
type CorpusTrip = {
  /// Its position in corpus.json, counting from 1.
  id: number;
  /// Set on the canonical trips at the head of the corpus.
  description?: string;
  /// Absent for trips not built around a roundabout.
  roundabout?: { lat: number; lon: number; name: string | null };
  from: LatLon;
  to: LatLon;
  after: { streets: string[] };
};

const props = defineProps<{ from?: LngLat; to?: LngLat }>();
const emit = defineEmits<{ select: [from: LngLat, to: LngLat] }>();

const corpus = ref<CorpusTrip[]>([]);
const lngLat = ([lat, lon]: LatLon) => new LngLat(lon, lat);
/// The corpus trip between the current endpoints, or -1.
const selected = computed(() => {
  const { from, to } = props;
  if (!from || !to) return -1;
  return corpus.value.findIndex(
    (t) =>
      lngLat(t.from).distanceTo(from) < SAME_PLACE_METERS &&
      lngLat(t.to).distanceTo(to) < SAME_PLACE_METERS,
  );
});

function title(t: CorpusTrip): string {
  if (t.description) return t.description;
  if (t.roundabout?.name) return t.roundabout.name;
  const named = t.after.streets.filter((n) => n && !GENERIC_STREETS.has(n));
  if (named.length === 0) {
    const [lat, lon] = t.roundabout
      ? [t.roundabout.lat, t.roundabout.lon]
      : t.from;
    return `${lat.toFixed(4)}, ${lon.toFixed(4)}`;
  }
  const first = named[0]!;
  const last = named[named.length - 1]!;
  return first === last ? first : `${first} → ${last}`;
}

function select(index: number) {
  const trip = corpus.value[index];
  if (trip) emit('select', lngLat(trip.from), lngLat(trip.to));
}

function step(delta: number) {
  const last = corpus.value.length - 1;
  select(Math.max(0, Math.min(last, selected.value + delta)));
}

function onKey(e: KeyboardEvent) {
  if (
    e.target instanceof HTMLInputElement ||
    e.target instanceof HTMLSelectElement
  )
    return;
  if (e.key === 'j') step(1);
  else if (e.key === 'k') step(-1);
}

onMounted(async () => {
  window.addEventListener('keydown', onKey);
  const trips: Omit<CorpusTrip, 'id'>[] = await fetch(CORPUS_URL).then((r) =>
    r.json(),
  );
  corpus.value = trips.map((t, i) => ({ ...t, id: i + 1 }));
});

onBeforeUnmount(() => window.removeEventListener('keydown', onKey));
</script>

<style lang="scss" scoped>
.trip-corpus {
  font-size: 12px;
  padding: 0 8px;
  margin-bottom: 8px;
}

.trip-corpus-controls {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
}

.trip-corpus-list {
  height: 200px;
  border: 1px solid rgba(0, 0, 0, 0.15);
}

.trip-corpus-row {
  height: 28px;
  line-height: 28px;
  padding: 0 6px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  cursor: pointer;
  &.selected {
    background: #e7f5ff;
  }
}

.k {
  color: #666;
}
</style>
