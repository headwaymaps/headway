<template>
  <div>
    <div style="display: flex">
      <search-box
        :hint="$t('search.from')"
        :style="{ flex: 1 }"
        :initial-place="fromPlace"
        :tabindex="1"
        @did-select-place="didSelectFromPlace"
      />
      <q-btn
        size="12px"
        :style="{ marginLeft: '0.5em', marginRight: 0 }"
        flat
        round
        color="dark"
        icon="gps_fixed"
        @click="didClickFromGps"
      />
    </div>
    <div v-if="vias.length > 0" class="trip-search-vias">
      <div v-for="(_, index) in vias" :key="index" class="trip-search-via">
        <svg class="trip-search-via-dot" height="18" width="18">
          <circle cx="9" cy="9" r="6" stroke-width="3" fill="white" />
        </svg>
        <span class="trip-search-via-label">
          {{ $t('route_edit_waypoint_$n', { n: index + 1 }) }}
        </span>
        <q-btn
          size="10px"
          flat
          round
          color="dark"
          icon="close"
          :aria-label="$t('route_edit_remove_waypoint')"
          @click="didRemoveVia(index)"
        />
      </div>
    </div>
    <div :style="{ marginTop: vias.length > 0 ? 0 : '16px', display: 'flex' }">
      <search-box
        :hint="$t('search.to')"
        :style="{ flex: 1 }"
        :initial-place="toPlace"
        :tabindex="2"
        @did-select-place="didSelectToPlace"
      />
      <q-btn
        size="12px"
        :style="{ marginLeft: '0.5em', marginRight: 0 }"
        flat
        round
        color="dark"
        icon="swap_vert"
        @click="didClickSwap"
      />
    </div>
    <travel-mode-bar
      style="margin-top: 16px"
      :current-mode="currentMode"
      :to-place="toPlace"
      :from-place="fromPlace"
    />
    <div v-if="isTransit()" style="margin-top: 8px; margin-bottom: -8px">
      <div v-if="searchTime || searchDate">
        <q-btn
          v-if="!arriveBy"
          :label="$t('trip_search_depart_at')"
          icon="arrow_drop_down"
          outline
          style="padding-left: 4px; padding-right: 8px; margin-right: 8px"
          dense
          size="sm"
          @click="didClickDepartAt"
        />
        <q-btn
          v-if="arriveBy"
          :label="$t('trip_search_arrive_by')"
          icon="arrow_drop_down"
          outline
          style="padding-left: 4px; padding-right: 8px; margin-right: 8px"
          dense
          size="sm"
          @click="didClickArriveBy"
        />
        <input
          type="time"
          :value="initialSearch.searchTime"
          @change="
            (event) => (searchTime = (event.target as HTMLInputElement).value)
          "
        />
        <input
          type="date"
          :value="initialSearch.searchDate"
          @change="
            (event) => (searchDate = (event.target as HTMLInputElement).value)
          "
        />
      </div>
      <q-btn
        v-else
        :label="$t('trip_search_depart_now')"
        icon="arrow_drop_down"
        outline
        style="padding-left: 4px; padding-right: 8px"
        dense
        size="sm"
        @click="didClickDepartNow"
      />
      <q-checkbox
        v-model="transitWithBicycle"
        :label="$t('trip_search_transit_with_bike')"
      />
    </div>
  </div>
</template>
<script lang="ts">
import { TravelMode } from 'src/utils/models';
import { defineComponent, PropType } from 'vue';
import TravelModeBar from 'src/components/TravelModeBar.vue';
import SearchBox from 'src/components/SearchBox.vue';
import Place from 'src/models/Place';
import { LngLat } from 'maplibre-gl';
import { placeDisplayName } from 'src/i18n/utils';
import env from 'src/utils/env';

export default defineComponent({
  name: 'TripSearch',
  components: { SearchBox, TravelModeBar },
  props: {
    fromPlace: {
      type: Place,
      default: undefined,
    },
    toPlace: {
      type: Place,
      default: undefined,
    },
    currentMode: { type: String as () => TravelMode, required: true },
    /// Places the route was dragged through, in order.
    vias: {
      type: Array as PropType<LngLat[]>,
      default: () => [],
    },
    didRemoveVia: {
      type: Function as PropType<(index: number) => void>,
      default: () => {},
    },
    didSelectFromPlace: {
      type: Function as PropType<(newValue?: Place) => void>,
      required: true,
    },
    didSelectToPlace: {
      type: Function as PropType<(newValue?: Place) => void>,
      required: true,
    },
    didSwapPlaces: {
      type: Function as PropType<
        (newToValue?: Place, newFromValue?: Place) => void
      >,
      required: true,
    },
    initialSearch: {
      type: Object as PropType<{
        searchTime?: string;
        searchDate?: string;
        arriveBy?: boolean;
        transitWithBicycle?: boolean;
      }>,
      required: true,
    },
    searchDidChange: {
      type: Function as PropType<
        (newValue: {
          searchTime?: string;
          searchDate?: string;
          arriveBy?: boolean;
          transitWithBicycle: boolean;
        }) => void
      >,
      required: true,
    },
  },
  data(): {
    searchTime?: string;
    searchDate?: string;
    arriveBy?: boolean;
    transitWithBicycle: boolean;
  } {
    return {
      searchTime: this.initialSearch.searchTime,
      searchDate: this.initialSearch.searchDate,
      arriveBy: this.initialSearch.arriveBy,
      transitWithBicycle: this.initialSearch.transitWithBicycle ?? false,
    };
  },
  watch: {
    searchTime: function () {
      this.searchDidChange(this.$data);
    },
    searchDate: function () {
      this.searchDidChange(this.$data);
    },
    arriveBy: function () {
      this.searchDidChange(this.$data);
    },
    transitWithBicycle: function () {
      this.searchDidChange(this.$data);
    },
  },
  methods: {
    isTransit(): boolean {
      return this.currentMode == TravelMode.Transit;
    },
    didClickDepartAt() {
      this.arriveBy = true;
    },
    didClickArriveBy() {
      this.arriveBy = false;
      this.searchTime = undefined;
      this.searchDate = undefined;
    },
    didClickDepartNow() {
      // BRITTLE: search date needs to be set first
      // beacuse OTP will error if a time is set without a date.
      this.searchDate = dateToInput(new Date());
      this.searchTime = timeToInput(new Date());
    },
    didClickSwap(event: Event) {
      (event.currentTarget as HTMLElement)
        .querySelector('.q-icon')
        ?.animate(
          [{ transform: 'rotate(0)' }, { transform: 'rotate(180deg)' }],
          {
            duration: 300,
            easing: 'ease-out',
          },
        );
      this.didSwapPlaces(this.toPlace, this.fromPlace);
    },
    didClickFromGps() {
      const options = {
        enableHighAccuracy: true,
        maximumAge: 60000,
        timeout: 10000,
      };

      env.geolocation.getCurrentPosition(
        (position: GeolocationPosition) => {
          const lngLat = new LngLat(
            position.coords.longitude,
            position.coords.latitude,
          );
          const place = Place.bareLocation(lngLat);
          place.name = this.$t('my_location');
          this.didSelectFromPlace(place);
        },
        console.error,
        options,
      );
    },
    placeDisplayName,
  },
});

function dateToInput(date: Date) {
  return (
    date.getFullYear() +
    '-' +
    ('0' + (date.getMonth() + 1)).slice(-2) +
    '-' +
    ('0' + date.getDate()).slice(-2)
  );
}

function timeToInput(date: Date) {
  return (
    ('0' + date.getHours()).slice(-2) +
    ':' +
    ('0' + date.getMinutes()).slice(-2)
  );
}
</script>

<style lang="scss" scoped>
// Palette.activeRoute, matching the route and its waypoints on the map.
$route-color: #1296ff;
$dot-center: 9px;

// A dashed stretch of route running from "from", through each waypoint, to "to".
.trip-search-vias {
  position: relative;
  padding: 8px 0;

  &::before {
    content: '';
    position: absolute;
    left: $dot-center - 2px;
    width: 4px;
    top: 0;
    bottom: 0;
    // Each tile is a dash with half a gap either side, centered so the line ends alike at
    // "from" and "to".
    background: linear-gradient(
        to bottom,
        transparent 2px,
        $route-color 2px 8px,
        transparent 8px
      )
      center / 4px 10px repeat-y;
  }
}

.trip-search-via {
  display: flex;
  align-items: center;
  gap: 8px;
}

.trip-search-via-dot {
  position: relative;
  flex-shrink: 0;

  circle {
    stroke: $route-color;
  }
}

.trip-search-via-label {
  flex: 1;
}
</style>
