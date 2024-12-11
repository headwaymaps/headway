<template>
  <div class="top-card">
    <search-box
      :initial-input-text="searchText"
      @did-select-place="searchBoxDidSelectPlace"
      @did-submit-search="searchBoxDidSubmitSearch"
    />
  </div>

  <div ref="bottomCard" class="bottom-card">
    <q-linear-progress v-if="isLoading" indeterminate />
    <div v-if="selectedPlace" class="selected-place-card">
      <place-card
        :place="selectedPlace"
        :did-press-close="
          () => {
            selectedPlace = undefined;
            boundMapToSearchResults();
          }
        "
      />
    </div>
    <q-list v-if="$q.screen.gt.sm || !selectedPlace" class="search-results">
      <search-list-item
        v-for="place in searchResults?.places"
        :id="`search-list-item-${place.id.serialized()}`"
        :key="place.id.serialized()"
        :place="place"
        :active="place == selectedPlace"
        clickable
        @mouseenter="didHoverSearchListItem(place)"
        @mouseleave="didHoverSearchListItem(undefined)"
        @click="didClickSearchListItem(place)"
      />
      <q-item
        v-if="searchResults?.places.length === 0 && !isLoading"
        class="list-item"
      >
        <q-item-section>
          <q-item-label>
            {{ $t('search_results_not_found_header') }}
          </q-item-label>
          <!-- eslint-disable vue/no-v-html vue/no-v-text-v-html-on-component -->
          <q-item-label
            class="text-weight-light"
            v-html="
              $t('search_results_not_found_subheader', {
                osmLink:
                  '<a href=https://www.openstreetmap.org>OpenStreetMap</a>',
              })
            "
          />
          <!-- eslint-enable vue/no-v-html vue/no-v-text-v-html-on-component -->
        </q-item-section>
      </q-item>
    </q-list>
  </div>
</template>

<script lang="ts">
import { baseMapPromise, getBaseMap } from 'src/components/BaseMap.vue';
import SearchBox from 'src/components/SearchBox.vue';
import SearchListItem from 'src/components/SearchListItem.vue';
import PlaceCard from 'src/components/PlaceCard.vue';
import Place, { PlaceId } from 'src/models/Place';
import PeliasClient from 'src/services/PeliasClient';
import Markers from 'src/utils/Markers';
import { supportsHover } from 'src/utils/misc';
import { defineComponent } from 'vue';
import { FlyToOptions, LngLatBoundsLike } from 'maplibre-gl';

type SearchResults = { places: Place[]; bbox: LngLatBoundsLike | undefined };

export default defineComponent({
  name: 'SearchPage',
  components: { PlaceCard, SearchBox, SearchListItem },
  props: {
    searchText: {
      type: String,
      required: true,
    },
  },
  data(): {
    searchResults?: SearchResults;
    selectedPlace?: Place;
    hoveredPlace?: Place;
    isLoading: boolean;
  } {
    return {
      searchResults: undefined,
      selectedPlace: undefined,
      hoveredPlace: undefined,
      isLoading: false,
    };
  },
  mounted(): void {
    this.updateSearch(this.searchText);
  },
  async unmounted(): Promise<void> {
    const map = await baseMapPromise;
    this.searchResults = undefined;
    map.removeAllMarkers();
  },
  methods: {
    searchBoxDidSubmitSearch(searchText: string): void {
      this.updateSearch(searchText);
      this.$router.replace(`/search/${encodeURIComponent(searchText)}`);
    },
    searchBoxDidSelectPlace(place?: Place): void {
      if (place) {
        this.$router.push(`/place/${place.urlEncodedId()}`);
      } else {
        // User "cleared" search field
        this.$router.push('/');
      }
    },
    didClickSearchListItem(place: Place): void {
      this.selectedPlace = place;
      this.renderPlacesOnMap();
      const map = getBaseMap();
      if (!map) {
        console.error('map was unexpectedly nil');
        return;
      }

      let options: FlyToOptions | undefined;

      if (!this.$refs.bottomCard) {
        console.error('bottomCard was unset');
        return;
      }

      const bottomCard: HTMLElement = this.$refs.bottomCard as HTMLElement;
      if (this.$q.screen.gt.sm) {
        // This abuses the fact that the "selected place card" is the same
        // width as the bottomCard. We could use $refs.selectedPlaceCard,
        // but it might not be visible to measure yet.
        const xOffset = bottomCard.offsetWidth;
        if (place.bbox) {
          options = { offset: [xOffset / 4, 0] };
        } else {
          options = { offset: [xOffset / 2, 0] };
        }
      }
      map.flyToPlace(place, options);
    },
    didHoverSearchListItem(place?: Place): void {
      if (!supportsHover()) {
        // FIX: selecting autocomplete item on mobile requires double
        // tapping.
        //
        // On touch devices, where hover is not supported, this method is
        // fired upon tapping. I don't fully understand why, but maybe
        // mutating the state in this method would rebuild the component,
        // canceling any outstanding event handlers on the old component.
        return;
      }
      this.hoveredPlace = place;
      this.renderPlacesOnMap();
    },
    didClickPlaceMarker(place: Place): void {
      this.selectedPlace = place;
      this.renderPlacesOnMap();
      const searchListItem = document.getElementById(
        `search-list-item-${place.id.serialized()}`,
      );
      if (searchListItem) {
        // This is async because we want to scroll after re-rendering the
        // "selected" cell, which is slightly larger.
        // Otherwise, if we scroll before the cell is re-rendered, it's new size
        // might be slightly out of view.
        setTimeout(() =>
          searchListItem.scrollIntoView({
            behavior: 'smooth',
            block: 'nearest',
          }),
        );
      }
    },
    async updateSearch(searchText: string): Promise<void> {
      if (searchText.length == 0) {
        this.searchResults = undefined;
        return;
      }

      const map = await baseMapPromise;

      let focus = undefined;
      if (map.getZoom() > 6) {
        focus = map.getCenter();
      }

      const places: Place[] = [];
      let bbox: LngLatBoundsLike | undefined = undefined;

      try {
        // The search endpoint results are worse for categorical searches like "coffee"
        // See: https://github.com/pelias/pelias/issues/938
        // const results = await PeliasClient.search(searchText, focus);
        //
        // So for now we're using autocomplete. Otherwise I think it's too weird
        // to show such different results.
        this.isLoading = true;
        this.searchResults = undefined;
        const results = await PeliasClient.autocomplete(
          searchText,
          focus,
        ).finally(() => {
          this.isLoading = false;
        });

        if (results.features.length == 0) {
          // no results
        } else if (!results.bbox) {
          console.error('search results missing bounding box');
        } else if (results.bbox.length != 4) {
          console.error('unexpected bbox dimensions');
        } else {
          bbox = results.bbox;
        }

        for (const feature of results.features) {
          if (!feature.properties?.gid) {
            console.error('feature was missing gid');
            continue;
          }
          const gid = feature.properties.gid;
          const id = PlaceId.gid(gid);
          places.push(Place.fromFeature(id, feature));
        }
      } catch (e) {
        console.warn('error with autocomplete', e);
      }

      this.searchResults = { places, bbox };
      this.renderPlacesOnMap();
      this.boundMapToSearchResults();
    },
    async boundMapToSearchResults() {
      const map = await baseMapPromise;
      if (this.searchResults?.bbox) {
        map.fitBounds(this.searchResults.bbox);
      }
    },
    renderPlacesOnMap() {
      const map = getBaseMap();
      if (!map) {
        console.error('basemap was unexpectedly unset');
        return;
      }

      map.removeAllMarkers();
      this.searchResults?.places.forEach((place: Place, idx: number) => {
        if (place == this.selectedPlace || place == this.hoveredPlace) {
          return;
        }
        const marker = Markers.inactive().setLngLat(place.point);
        marker.getElement().addEventListener('click', () => {
          this.didClickPlaceMarker(place);
        });
        map.pushMarker(`place_${idx}`, marker);
      });

      if (this.selectedPlace) {
        const marker = Markers.active().setLngLat(this.selectedPlace.point);
        map.pushMarker('selected_place', marker);
      }
      if (this.hoveredPlace) {
        const marker = Markers.active().setLngLat(this.hoveredPlace.point);
        map.pushMarker('hovered_place', marker);
      }
    },
  },
});
</script>

<style lang="scss">
.selected-place-card {
  @media screen and (min-width: 800px) {
    // on "desktop" layout
    position: absolute;
    z-index: 1;
    left: var(--left-panel-width);
    margin-top: 16px;
    margin-left: 16px;
    border-radius: 4px;
    width: var(--left-panel-width);
    box-shadow: 0px 0px 5px #00000088;
    background-color: white;
  }
}
</style>
