<template>
  <div class="top-card">
    <search-box
      :tabindex="1"
      @did-select-place="searchBoxDidSelectPlace"
      @did-submit-search="
        (searchText) =>
          $router.push(`/search/${encodeURIComponent(searchText)}`)
      "
    />
  </div>

  <!-- Lives inside #map so its coordinates are the ones project() and
       unproject() speak. Positioned against the page instead, it would be off
       by the width of the left panel. Re-placed whenever the map moves, so it
       stays over the same ground. -->
  <teleport v-if="mapMounted" to="#map">
    <svg class="zone-feed-boxes" aria-hidden="true">
      <polygon
        v-for="box in selectedFeedBoxes"
        :key="box.feedId"
        :points="box.points"
        class="zone-feed-box"
      />
      <polygon
        v-if="highlightedFeedBox"
        :points="highlightedFeedBox.points"
        class="zone-feed-box zone-feed-box-highlighted"
      />
    </svg>
    <div v-show="boxStyle" class="zone-box" :style="boxStyle"></div>
  </teleport>

  <div class="bottom-card zone-panel">
    <header>
      <h1>Transit zone</h1>
      <p v-if="!area">
        Search for a place, then drag a box over the area you want to serve.
      </p>
      <p v-else-if="loading">Looking for feeds…</p>
      <p v-else-if="error" class="zone-error">{{ error }}</p>
      <p v-else>
        {{ feeds.length }} feeds touch this area; {{ selected.size }} selected.
      </p>
    </header>

    <!-- Drawing is the action that produces everything else, so it stands
         alone; selecting within a result is a refinement of it. -->
    <div class="zone-controls">
      <q-btn
        dense
        no-caps
        outline
        color="primary"
        :icon="drawing ? 'close' : 'crop_free'"
        :label="drawing ? 'Cancel' : area ? 'Redraw area' : 'Draw area'"
        @click="setDrawing(!drawing)"
      />
    </div>

    <div v-if="feeds.length" class="zone-select-tools">
      <span class="zone-select-label">Select</span>
      <button type="button" @click="selectAll">all</button>
      <span aria-hidden="true">·</span>
      <button type="button" @click="selectNone">none</button>
    </div>

    <ul class="zone-feeds">
      <li
        v-for="feed in feeds"
        :key="feed.feed_id"
        @mouseenter="hoveredFeedId = feed.feed_id"
        @mouseleave="hoveredFeedId = null"
        @focusin="focusedFeedId = feed.feed_id"
        @focusout="focusedFeedId = null"
      >
        <q-checkbox
          dense
          :model-value="selected.has(feed.feed_id)"
          @update:model-value="toggle(feed.feed_id)"
        >
          <span class="zone-feed-provider">{{ feed.provider }}</span>
          <span class="zone-feed-id">{{ feed.feed_id }}</span>
          <span
            class="zone-feed-metrics"
            title="Area of the feed's bounding box, not its actual service coverage"
          >
            {{ formatArea(feed.area_m2) }} km² extent
          </span>
          <span v-if="feed.realtime.length" class="zone-feed-rt">
            realtime: {{ realtimeKinds(feed) }}
          </span>
          <span
            v-if="feed.authorization_type !== 'none'"
            class="zone-feed-auth"
            :title="'needs a credential: ' + feed.authorization_type"
          >
            needs a token
          </span>
        </q-checkbox>
      </li>
      <li v-if="area && !loading && !feeds.length" class="zone-empty">
        No measured feeds overlap this area.
      </li>
    </ul>

    <footer>
      <q-btn
        dense
        no-caps
        color="primary"
        label="Download zone.json"
        :disable="!selected.size"
        @click="download"
      />
    </footer>
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import { getBaseMap, baseMapPromise } from 'src/components/BaseMap.vue';
import SearchBox from 'src/components/SearchBox.vue';
import Place from 'src/models/Place';
import TransitZonerClient, {
  Bbox,
  FeedSummary,
} from 'src/services/TransitZonerClient';

/// A rectangle in screen pixels, as the pointer draws it.
interface ScreenRect {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

interface FeedBox {
  feedId: string;
  points: string;
}

export default defineComponent({
  name: 'TransitZonePage',
  components: { SearchBox },
  data: function (): {
    mapMounted: boolean;
    drawing: boolean;
    dragStart: { x: number; y: number } | null;
    screenRect: ScreenRect | null;
    area: Bbox | null;
    feeds: FeedSummary[];
    selected: Set<string>;
    feedBoxes: FeedBox[];
    hoveredFeedId: string | null;
    focusedFeedId: string | null;
    loading: boolean;
    error: string | null;
    requestSeq: number;
  } {
    return {
      mapMounted: false,
      drawing: false,
      dragStart: null,
      screenRect: null,
      area: null,
      feeds: [],
      selected: new Set(),
      feedBoxes: [],
      hoveredFeedId: null,
      focusedFeedId: null,
      loading: false,
      error: null,
      requestSeq: 0,
    };
  },
  computed: {
    selectedFeedBoxes(): FeedBox[] {
      return this.feedBoxes.filter((box) => this.selected.has(box.feedId));
    },
    highlightedFeedBox(): FeedBox | undefined {
      return this.feedBoxes.find(
        (box) => box.feedId === (this.hoveredFeedId ?? this.focusedFeedId),
      );
    },
    boxStyle(): Record<string, string> | null {
      const rect = this.screenRect;
      if (!rect) {
        return null;
      }
      return {
        left: `${Math.min(rect.x1, rect.x2)}px`,
        top: `${Math.min(rect.y1, rect.y2)}px`,
        width: `${Math.abs(rect.x2 - rect.x1)}px`,
        height: `${Math.abs(rect.y2 - rect.y1)}px`,
      };
    },
  },
  watch: {
    feeds() {
      this.hoveredFeedId = null;
      this.focusedFeedId = null;
      this.renderBox();
    },
  },
  mounted: async function () {
    const baseMap = await baseMapPromise;
    baseMap.removeAllMarkers();
    this.mapMounted = document.getElementById('map') !== null;
    this.renderBox();
    // The drawn area is in world coordinates; its box is in screen ones, so it
    // has to be re-placed on every frame the map moves.
    baseMap.onMapEvent('move', this.renderBox);

    const container = document.getElementById('map');
    container?.addEventListener('mousedown', this.onMouseDown);
    window.addEventListener('mousemove', this.onMouseMove);
    window.addEventListener('mouseup', this.onMouseUp);
  },
  unmounted: function () {
    const baseMap = getBaseMap();
    baseMap?.offMapEvent('move', this.renderBox);
    baseMap?.setDragPan(true);
    document
      .getElementById('map')
      ?.removeEventListener('mousedown', this.onMouseDown);
    window.removeEventListener('mousemove', this.onMouseMove);
    window.removeEventListener('mouseup', this.onMouseUp);
  },
  methods: {
    searchBoxDidSelectPlace(place?: Place) {
      if (place) {
        getBaseMap()?.flyToPlace(place);
      }
    },
    setDrawing(on: boolean) {
      // Starting a new drawing drops the old one: the box and the feed list
      // both describe the previous area, and leaving either on screen while
      // drawing the next one says they describe the new area.
      if (on) {
        this.area = null;
        this.screenRect = null;
        this.feeds = [];
        this.selected = new Set();
        this.error = null;
        // Any in-flight lookup is for the area just discarded.
        this.requestSeq += 1;
        this.loading = false;
      }
      this.drawing = on;
      // Dragging to draw and dragging to pan are the same gesture.
      getBaseMap()?.setDragPan(!on);
      getBaseMap()?.setCursor(on ? 'crosshair' : '');
    },
    realtimeKinds(feed: FeedSummary): string {
      return [...new Set(feed.realtime.flatMap((rt) => rt.kinds))].join(', ');
    },
    formatArea(squareMeters: number): string {
      const squareKilometers = squareMeters / 1_000_000;
      if (squareKilometers > 0 && squareKilometers < 0.1) {
        return '<0.1';
      }
      return squareKilometers.toLocaleString(undefined, {
        maximumFractionDigits: 1,
      });
    },
    onMouseDown(event: MouseEvent) {
      if (!this.drawing) {
        return;
      }
      event.preventDefault();
      const point = this.pointIn(event);
      this.dragStart = point;
      this.screenRect = { x1: point.x, y1: point.y, x2: point.x, y2: point.y };
    },
    onMouseMove(event: MouseEvent) {
      if (!this.dragStart) {
        return;
      }
      const point = this.pointIn(event);
      this.screenRect = {
        x1: this.dragStart.x,
        y1: this.dragStart.y,
        x2: point.x,
        y2: point.y,
      };
    },
    onMouseUp() {
      if (!this.dragStart || !this.screenRect) {
        return;
      }
      const rect = this.screenRect;
      this.dragStart = null;
      // A click rather than a drag: nothing was drawn.
      if (Math.abs(rect.x2 - rect.x1) < 4 || Math.abs(rect.y2 - rect.y1) < 4) {
        this.screenRect = null;
        return;
      }
      this.setDrawing(false);
      this.area = this.bboxFor(rect);
      void this.loadFeeds();
    },
    /// Pointer position relative to the map container, which is what project()
    /// and unproject() speak.
    pointIn(event: MouseEvent): { x: number; y: number } {
      const bounds = document.getElementById('map')?.getBoundingClientRect();
      return {
        x: event.clientX - (bounds?.left ?? 0),
        y: event.clientY - (bounds?.top ?? 0),
      };
    },
    bboxFor(rect: ScreenRect): Bbox | null {
      const baseMap = getBaseMap();
      if (!baseMap) {
        return null;
      }
      const a = baseMap.unproject([rect.x1, rect.y1]);
      const b = baseMap.unproject([rect.x2, rect.y2]);
      return [
        Math.min(a.lng, b.lng),
        Math.min(a.lat, b.lat),
        Math.max(a.lng, b.lng),
        Math.max(a.lat, b.lat),
      ];
    },
    /// Put the box back over the ground it was drawn on.
    renderBox() {
      const baseMap = getBaseMap();
      if (!baseMap) {
        return;
      }
      // Project all four corners so feed extents also follow map rotation
      // and pitch. Selection and hover reuse these screen coordinates.
      this.feedBoxes = this.feeds.map((feed) => {
        const [west, south, east, north] = feed.bbox;
        const corners: [number, number][] = [
          [west, north],
          [east, north],
          [east, south],
          [west, south],
        ];
        return {
          feedId: feed.feed_id,
          points: corners
            .map((corner) => {
              const point = baseMap.project(corner);
              return `${point.x},${point.y}`;
            })
            .join(' '),
        };
      });
      if (!this.area || this.dragStart) {
        return;
      }
      const [minLon, minLat, maxLon, maxLat] = this.area;
      const topLeft = baseMap.project([minLon, maxLat]);
      const bottomRight = baseMap.project([maxLon, minLat]);
      this.screenRect = {
        x1: topLeft.x,
        y1: topLeft.y,
        x2: bottomRight.x,
        y2: bottomRight.y,
      };
    },
    async loadFeeds() {
      if (!this.area) {
        return;
      }
      // Panning while a request is in flight can land an older response after a
      // newer one; only the newest may write.
      const seq = ++this.requestSeq;
      this.loading = true;
      this.error = null;
      try {
        const feeds = await TransitZonerClient.feedsByBbox(this.area);
        if (seq !== this.requestSeq) {
          return;
        }
        this.feeds = feeds;
        this.selected = new Set(feeds.map((feed) => feed.feed_id));
      } catch (e) {
        if (seq === this.requestSeq) {
          this.error = e instanceof Error ? e.message : String(e);
          this.feeds = [];
          this.selected = new Set();
        }
      } finally {
        if (seq === this.requestSeq) {
          this.loading = false;
        }
      }
    },
    toggle(feedId: string) {
      const next = new Set(this.selected);
      if (next.has(feedId)) {
        next.delete(feedId);
      } else {
        next.add(feedId);
      }
      this.selected = next;
    },
    selectAll() {
      this.selected = new Set(this.feeds.map((feed) => feed.feed_id));
    },
    selectNone() {
      this.selected = new Set();
    },
    async download() {
      if (!this.area || !this.selected.size) {
        return;
      }
      try {
        const json = await TransitZonerClient.zoneDocument(this.area, [
          ...this.selected,
        ]);
        const url = URL.createObjectURL(
          new Blob([json], { type: 'application/json' }),
        );
        const link = document.createElement('a');
        link.href = url;
        link.download = 'zone.json';
        link.click();
        URL.revokeObjectURL(url);
      } catch (e) {
        this.error = e instanceof Error ? e.message : String(e);
      }
    },
  },
});
</script>

<style lang="scss">
.zone-feed-boxes {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  z-index: 3;
  pointer-events: none;
}

.zone-feed-box {
  fill: rgba(0, 137, 123, 0.04);
  stroke: #00897b;
  stroke-width: 2;
}

.zone-feed-box-highlighted {
  fill: rgba(239, 108, 0, 0.16);
  stroke: #ef6c00;
  stroke-width: 4;
}

// Inside #map, which maplibre positions, so these are map coordinates.
.zone-box {
  position: absolute;
  z-index: 2;
  border: 2px solid var(--q-primary);
  background: rgba(25, 118, 210, 0.12);
  pointer-events: none;
}

// Layout comes from .bottom-card: the left column below the search box, which
// is also what keeps the autocomplete menu above this rather than behind it.
.zone-panel {
  background: white;

  header {
    padding: 8px 16px 0;

    h1 {
      font-size: 1.1rem;
      line-height: 1.4;
      margin: 0;
    }

    p {
      margin: 4px 0;
      font-size: 0.85rem;
      color: #555;
    }
  }

  .zone-error {
    color: #b00020;
  }

  .zone-controls {
    display: flex;
    gap: 4px;
    padding: 4px 16px 8px;

    // The icon+label pair sits tight against the right edge by default.
    .q-btn {
      padding-right: 12px;
    }
  }

  // Secondary to the draw button: this only refines a result that already
  // exists, so it reads as part of the list rather than as a tool of its own.
  .zone-select-tools {
    display: flex;
    align-items: baseline;
    gap: 4px;
    padding: 4px 16px;
    font-size: 0.75rem;
    color: #777;
    border-top: solid #eee 1px;

    button {
      background: none;
      border: none;
      padding: 0;
      font: inherit;
      color: var(--q-primary);
      cursor: pointer;
      text-decoration: underline;
    }
  }

  .zone-feeds {
    list-style: none;
    margin: 0;
    padding: 4px 8px;

    li {
      padding: 2px 0;
    }
  }

  .zone-feed-provider {
    font-weight: 500;
  }

  .zone-feed-id,
  .zone-feed-metrics,
  .zone-feed-rt,
  .zone-feed-auth {
    display: block;
    font-size: 0.75rem;
    color: #666;
  }

  .zone-feed-auth {
    color: #8a6d00;
  }

  .zone-empty {
    color: #666;
    font-size: 0.85rem;
    padding: 8px 4px;
  }

  footer {
    padding: 8px 16px;
    border-top: solid #eee 1px;
    position: sticky;
    bottom: 0;
    background: white;
  }
}
</style>
