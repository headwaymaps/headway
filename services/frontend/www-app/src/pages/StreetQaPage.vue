<template>
  <div class="street-qa">
    <nav>
      <header class="row no-wrap items-center q-gutter-x-sm">
        <q-btn
          dense
          size="sm"
          no-caps
          outline
          label="New case"
          icon="add"
          @click="openDraft"
        />
        <a :href="links.google" target="_blank">Google Maps ↗</a>
        <a :href="links.satellite" target="_blank">satellite ↗</a>
        <a :href="links.osm" target="_blank">OSM ↗</a>
        <q-icon name="info_outline" size="xs" color="primary">
          <q-tooltip class="text-body2">
            <kbd>j</kbd> / <kbd>↓</kbd> next case<br />
            <kbd>k</kbd> / <kbd>↑</kbd> previous case<br />
            <kbd>o</kbd> mark good<br />
            <kbd>x</kbd> request a fix, saving the map view
          </q-tooltip>
        </q-icon>
        <div v-if="offline" class="text-negative">
          Status server not running, not saved.
        </div>
      </header>
      <q-form
        v-if="draft"
        class="case active q-gutter-y-xs"
        @submit="saveDraft"
        @keydown.stop
      >
        <q-input
          v-model="draft.title"
          dense
          outlined
          autofocus
          bg-color="white"
          placeholder="Title, e.g. street names"
          :rules="[(title: string) => !!title.trim() || 'Needs a title']"
        />
        <q-input
          v-model="draft.reported"
          dense
          outlined
          autogrow
          type="textarea"
          bg-color="white"
          placeholder="What's wrong? Center it under the crosshair; the case saves this view."
        />
        <div class="q-gutter-x-sm">
          <q-btn
            dense
            size="sm"
            no-caps
            unelevated
            color="primary"
            type="submit"
            label="Save"
          />
          <q-btn
            size="sm"
            dense
            no-caps
            flat
            label="Cancel"
            @click="draft = null"
          />
        </div>
      </q-form>
      <section v-for="(group, g) in groups" :key="g">
        <div
          v-for="{ qaCase, i } in group"
          :key="qaCase.title"
          :data-case="i"
          class="case"
          :class="{ active: i === current, folded: folded(qaCase) }"
          @click="open(i)"
        >
          <h3>
            <span class="n">{{ i + 1 }}</span
            >{{ qaCase.title }}
            <span
              v-if="statusOf(qaCase).ready && i !== current"
              class="verdict text-primary text-weight-bold"
              >● feedback requested</span
            >
            <span v-else class="verdict q-gutter-x-xs">
              <q-btn
                dense
                size="sm"
                label="👍"
                :outline="vote(qaCase) !== 'ok'"
                :color="vote(qaCase) === 'ok' ? 'positive' : 'grey-6'"
                @click.stop="judge(i, 'ok')"
              >
                <q-tooltip>Looks good</q-tooltip>
              </q-btn>
              <q-btn
                dense
                size="sm"
                label="👎"
                :outline="vote(qaCase) !== 'bad'"
                :color="vote(qaCase) === 'bad' ? 'negative' : 'grey-6'"
                @click.stop="judge(i, 'bad')"
              >
                <q-tooltip>Fix this, with my note as context</q-tooltip>
              </q-btn>
              <q-btn
                dense
                flat
                size="sm"
                icon="delete_outline"
                color="grey-6"
                @click.stop="remove(i)"
              >
                <q-tooltip>Delete this case</q-tooltip>
              </q-btn>
            </span>
          </h3>
          <p>{{ qaCase.reported }}</p>
          <p
            v-if="statusOf(qaCase).ready && statusOf(qaCase).claude"
            class="text-primary"
          >
            Claude: {{ statusOf(qaCase).claude }}
          </p>
          <q-input
            :model-value="noteOf(qaCase)"
            dense
            outlined
            autogrow
            type="textarea"
            bg-color="white"
            placeholder="Note for Claude"
            @click.stop
            @keydown.stop
            @update:model-value="
              (note) => (notes[qaCase.title] = String(note ?? ''))
            "
            @blur="save(qaCase.title, { note: noteOf(qaCase) })"
          />
        </div>
      </section>
    </nav>
    <main>
      <iframe ref="frame" title="Map"></iframe>
      <div class="crosshair"></div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref } from 'vue';

/** A spot where streets drew wrong, as reported. */
interface QaCase {
  title: string;
  /** The map's `zoom/lat/lng`. */
  view: string;
  reported: string;
}

type Verdict = 'ok' | 'bad';

/** What the user and Claude have said about a case, kept by the status server. */
interface CaseStatus {
  verdict?: Verdict | null;
  note?: string;
  ready?: boolean;
  claude?: string;
  /** Where the user was looking when they asked for a fix. */
  view?: string;
  /** Set on cases added from this page, which have no built-in entry. */
  added?: boolean;
  deleted?: boolean;
  reported?: string;
}

const BUILT_IN_CASES: QaCase[] = [
  {
    title: 'Alaskan Way at Marion',
    view: '19.26/47.6036096/-122.3377793',
    reported:
      'Dual carriageway halves merging into the single road: curb width jumps, halves too narrow then too wide before the join. Choppy at Madison too.',
  },
  {
    title: 'Boyer Ave E / 16th Ave E / E Lynn St',
    view: '18.44/47.6394818/-122.3107177',
    reported:
      'End caps of many short streets protrude past the streets they merge into.',
  },
  {
    title: 'Montlake Blvd NE short link',
    view: '18.36/47.653992/-122.302385',
    reported: 'Goofy end cap on a short segment between the carriageways.',
  },
  {
    title: 'E Lake Washington Blvd split',
    view: '18.37/47.644231/-122.302773',
    reported: 'Dual carriageway joining with a chop on its south curb.',
  },
  {
    title: 'Aurora Ave N / SR 99 ramps',
    view: '17.92/47.622653/-122.343698',
    reported: 'Lots of chop and bulges where ramps and halves overlap.',
  },
  {
    title: '7th Ave crossing Denny into Wall St',
    view: '19.15/47.6186454/-122.3436024',
    reported: 'Jagged tip.',
  },
  {
    title: '4th Ave & Bell St',
    view: '19/47.6153048/-122.3437909',
    reported: 'Width step at the intersection.',
  },
  {
    title: 'Denny Way at Terry / Boren',
    view: '19.31/47.6186075/-122.3364512',
    reported: 'Pinches in Denny between intersections.',
  },
  {
    title: 'Denny Way at Minor Ave',
    view: '18.72/47.6187253/-122.3326527',
    reported: 'Hard width step across Denny.',
  },
  {
    title: 'S Jackson St at 2nd Ave Ext S / 4th Ave S',
    view: '19.18/47.5991137/-122.3296098',
    reported: 'Casing drawn across the middle of the intersection.',
  },
  {
    title: 'E James St tile seam',
    view: '18.57/47.6079199/-122.3183909',
    reported: 'Seam where a wide street crossed a tile edge.',
  },
  {
    title: 'Sidewalks and cycle track downtown',
    view: '19.11/47.606051/-122.33266',
    reported: 'Choppy sidewalks, maybe the cycletrack.',
  },
  {
    title: 'Sidewalk corner at 13th Ave E & E Republican St',
    view: '18.45/47.6232366/-122.3158149',
    reported:
      'Sidewalk nearest the intersection is square and butts out; should curve with the curb.',
  },
];

/** Keeps verdicts and notes in a JSON file Claude reads. */
const STATUS_URL = 'http://127.0.0.1:8091/';

const statuses = ref<Record<string, CaseStatus>>({});
const offline = ref(false);
/** Notes as typed, ahead of the status server. */
const notes = reactive<Record<string, string>>({});
/** Good cases opened again by clicking them, until they're judged again. */
const unfolded = reactive(new Set<string>());
const current = ref(0);
const view = ref('');
const draft = ref<{ title: string; reported: string } | null>(null);
const frame = ref<HTMLIFrameElement>();

/** Built-in cases, then the ones added from this page, each at the view its fix was asked from. */
const cases = computed<QaCase[]>(() => {
  const added = Object.entries(statuses.value)
    .filter(([, status]) => status.added && status.view)
    .map(([title, status]) => ({
      title,
      view: status.view ?? '',
      reported: status.reported ?? '',
    }));
  return [...BUILT_IN_CASES, ...added]
    .filter((qaCase) => !statuses.value[qaCase.title]?.deleted)
    .map((qaCase) => ({
      ...qaCase,
      view: statuses.value[qaCase.title]?.view ?? qaCase.view,
    }));
});

/** Cases awaiting the user's vote, then those to fix, then the good ones, each with its number. */
const groups = computed(() => {
  const numbered = cases.value.map((qaCase, i) => ({ qaCase, i }));
  const rank = (qaCase: QaCase) =>
    vote(qaCase) === 'bad' ? 1 : vote(qaCase) === 'ok' ? 2 : 0;
  return [0, 1, 2]
    .map((g) => numbered.filter(({ qaCase }) => rank(qaCase) === g))
    .filter((group) => group.length > 0);
});

const links = computed(() => {
  const [zoom, lat, lng] = view.value.split('/');
  // Google's and OSM's zooms count 256 px tiles, MapLibre's 512.
  const google = `https://www.google.com/maps/@${lat},${lng},${(Number(zoom) + 1).toFixed(2)}z`;
  return {
    google,
    satellite: `${google}/data=!3m1!1e3`,
    osm: `https://www.openstreetmap.org/#map=${Math.round(Number(zoom) + 1)}/${lat}/${lng}`,
  };
});

function statusOf(qaCase: QaCase): CaseStatus {
  return statuses.value[qaCase.title] ?? {};
}

function noteOf(qaCase: QaCase): string {
  return notes[qaCase.title] ?? statusOf(qaCase).note ?? '';
}

/** The user's verdict, unless Claude has asked them to look again. */
function vote(qaCase: QaCase): Verdict | null | undefined {
  return statusOf(qaCase).ready ? undefined : statusOf(qaCase).verdict;
}

function folded(qaCase: QaCase): boolean {
  return vote(qaCase) === 'ok' && !unfolded.has(qaCase.title);
}

async function load() {
  try {
    statuses.value = await (await fetch(STATUS_URL)).json();
    offline.value = false;
  } catch {
    offline.value = true;
  }
}

async function save(title: string, fields: CaseStatus) {
  statuses.value = {
    ...statuses.value,
    [title]: { ...statuses.value[title], ...fields },
  };
  try {
    const response = await fetch(STATUS_URL, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ title, ...fields }),
    });
    statuses.value = await response.json();
  } catch {
    offline.value = true;
  }
}

function show(mapView: string) {
  const hash = `#map=${mapView}`;
  const map = frame.value?.contentWindow;
  if (map?.location.pathname === '/') {
    map.location.hash = hash;
  } else if (frame.value) {
    frame.value.src = `/${hash}`;
  }
}

function select(i: number, mapView?: string) {
  current.value = Math.max(0, Math.min(cases.value.length - 1, i));
  show(mapView ?? cases.value[current.value]!.view);
  void nextTick(() =>
    document
      .querySelector(`[data-case="${current.value}"]`)
      ?.scrollIntoView({ block: 'nearest' }),
  );
}

/** Selects the case `step` places from the current one, in the order they're listed. */
function step(step: number) {
  const order = groups.value.flat().map(({ i }) => i);
  const next = order[order.indexOf(current.value) + step];
  if (next !== undefined) {
    select(next);
  }
}

function open(i: number) {
  unfolded.add(cases.value[i]!.title);
  select(i);
}

/** Records a verdict; asking for a fix also saves where the map is, to point at the problem. */
function judge(i: number, verdict: Verdict) {
  const qaCase = cases.value[i]!;
  unfolded.delete(qaCase.title);
  const fields: CaseStatus = {
    verdict,
    note: noteOf(qaCase),
    ready: false,
  };
  if (verdict === 'bad' && i === current.value && view.value) {
    fields.view = view.value;
  }
  void save(qaCase.title, fields);
}

function toggle(verdict: Verdict) {
  const qaCase = cases.value[current.value]!;
  if (vote(qaCase) === verdict) {
    void save(qaCase.title, { verdict: null, ready: false });
  } else {
    judge(current.value, verdict);
  }
}

async function remove(i: number) {
  const qaCase = cases.value[i]!;
  if (!confirm(`Delete "${qaCase.title}"?`)) {
    return;
  }
  await save(qaCase.title, { deleted: true });
  select(current.value > i ? current.value - 1 : current.value);
}

function openDraft() {
  draft.value = { title: '', reported: '' };
}

async function saveDraft() {
  const title = draft.value?.title.trim();
  if (!title || !view.value || cases.value.some((c) => c.title === title)) {
    return;
  }
  await save(title, {
    added: true,
    view: view.value,
    reported: draft.value?.reported ?? '',
    verdict: 'bad',
    ready: false,
    deleted: false,
  });
  draft.value = null;
  select(cases.value.findIndex((c) => c.title === title));
}

function onKey(event: KeyboardEvent) {
  if (event.key === 'j' || event.key === 'ArrowDown') {
    step(1);
  } else if (event.key === 'k' || event.key === 'ArrowUp') {
    step(-1);
  } else if (event.key === 'o') {
    toggle('ok');
  } else if (event.key === 'x') {
    toggle('bad');
  } else {
    return;
  }
  event.preventDefault();
}

/** Follows the map's view, and keeps this page's URL pointing at the case and view. */
function track() {
  try {
    view.value =
      frame.value?.contentWindow?.location.hash.replace(/^#map=/, '') ?? '';
  } catch {
    view.value = '';
  }
  const hash = `#case=${current.value + 1}${view.value ? `&map=${view.value}` : ''}`;
  if (location.hash !== hash) {
    history.replaceState(history.state, '', hash);
  }
}

let timers: ReturnType<typeof setInterval>[] = [];

onMounted(async () => {
  const linked = new URLSearchParams(location.hash.slice(1));
  await load();
  select(Number(linked.get('case') || 1) - 1, linked.get('map') ?? undefined);
  document.addEventListener('keydown', onKey);
  timers = [setInterval(track, 500), setInterval(load, 3000)];
});

onUnmounted(() => {
  document.removeEventListener('keydown', onKey);
  timers.forEach(clearInterval);
});
</script>

<style scoped>
.street-qa {
  --line: #ddd;
  --active: #e8eefc;
  position: fixed;
  inset: 0;
  display: flex;
  background: #fafaf8;
}
section + section {
  border-top: 6px solid #e4e4e0;
}
nav {
  width: 340px;
  overflow-y: auto;
  border-right: 1px solid var(--line);
}
header {
  position: sticky;
  top: 0;
  z-index: 1;
  background: #fafaf8;
  padding: 8px 14px;
  border-bottom: 1px solid var(--line);
  font-size: 12px;
}
kbd {
  display: inline-block;
  min-width: 18px;
  padding: 0 4px;
  border-radius: 3px;
  background: #555;
  text-align: center;
  font-family: monospace;
}
.case {
  padding: 10px 14px;
  border-bottom: 1px solid var(--line);
  cursor: pointer;
}
.case.active {
  background: var(--active);
}
.case h3 {
  margin: 0 0 4px;
  font-size: 14px;
  line-height: 1.4;
  font-weight: bold;
  display: flex;
  gap: 6px;
  align-items: baseline;
}
.case .n {
  color: #666;
  font-weight: normal;
}
.case p {
  margin: 2px 0 6px;
  color: #666;
  font-size: 12px;
}
.case.folded {
  padding: 4px 14px;
}
.case.folded h3 {
  margin: 0;
  font-size: 12px;
  font-weight: normal;
  color: #666;
}
.case.folded > :not(h3) {
  display: none;
}
.case :deep(textarea),
.case :deep(input) {
  font-size: 12px;
  line-height: 1.4;
}
.verdict {
  margin-left: auto;
  font-size: 12px;
  font-weight: normal;
  white-space: nowrap;
}
main {
  flex: 1;
  position: relative;
}
iframe {
  border: 0;
  width: 100%;
  height: 100%;
}
.crosshair {
  position: absolute;
  left: 50%;
  top: 50%;
  width: 28px;
  height: 28px;
  margin: -14px 0 0 -14px;
  pointer-events: none;
}
.crosshair::before,
.crosshair::after {
  content: '';
  position: absolute;
  background: #e0218a;
}
.crosshair::before {
  left: 13px;
  top: 0;
  width: 2px;
  height: 28px;
}
.crosshair::after {
  top: 13px;
  left: 0;
  height: 2px;
  width: 28px;
}
@media (max-width: 700px) {
  .street-qa {
    flex-direction: column;
  }
  nav {
    width: 100%;
    height: 40vh;
    border-right: 0;
    border-bottom: 1px solid var(--line);
  }
}
</style>
