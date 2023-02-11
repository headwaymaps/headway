<template>
  <div class="top-card">
    <search-box
      :initial-input-text="searchText"
      v-on:did-select-place="searchBoxDidSelectPlace"
      v-on:did-submit-search="searchBoxDidSubmitSearch"
    />
  </div>

  <div class="bottom-card">
    <q-linear-progress v-if="isLoading" indeterminate />
    <q-list>
      <search-list-item
        v-for="place in placeChoices"
        v-bind:key="place.id.serialized()"
        :place="place"
        :active="place == selectedPlace"
        clickable
        :id="`search-list-item-${place.id.serialized()}`"
        v-on:mouseenter="didHoverSearchListItem(place)"
        v-on:mouseleave="didHoverSearchListItem(undefined)"
        v-on:click="didClickSearchListItem(place)"
      />
    </q-list>
  </div>
</template>

<script lang="ts">
import { baseMapPromise, getBaseMap } from 'src/components/BaseMap.vue';
import SearchBox from 'src/components/SearchBox.vue';
import SearchListItem from 'src/components/SearchListItem.vue';
import Place, { PlaceId } from 'src/models/Place';
import PeliasClient from 'src/services/PeliasClient';
import Markers from 'src/utils/Markers';
import { supportsHover } from 'src/utils/misc';
import { defineComponent } from 'vue';

export default defineComponent({
  name: 'SearchPage',
  props: {
    searchText: {
      type: String,
      required: true,
    },
  },
  data(): {
    placeChoices: Place[];
    selectedPlace?: Place;
    hoveredPlace?: Place;
    isLoading: boolean;
  } {
    return {
      placeChoices: [],
      selectedPlace: undefined,
      hoveredPlace: undefined,
      isLoading: false,
    };
  },
  components: { SearchBox, SearchListItem },
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
      let map = getBaseMap();
      if (!map) {
        console.error('map was unexpectedly nil');
        return;
      }
      map.flyToPlace(place);
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
        `search-list-item-${place.id.serialized()}`
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
          })
        );
      }
    },
    async updateSearch(searchText: string): Promise<void> {
      if (searchText.length == 0) {
        this.placeChoices = [];
        return;
      }

      let map = await baseMapPromise;

      let focus = undefined;
      if (map.getZoom() > 6) {
        focus = map.getCenter();
      }

      let places: Place[] = [];
      try {
        // The search endpoint results are worse for categorical searches like "coffee"
        // See: https://github.com/pelias/pelias/issues/938
        // const results = await PeliasClient.search(searchText, focus);
        //
        // So for now we're using autocomplete. Otherwise I think it's too weird
        // to show such different results.
        this.isLoading = true;
        this.placeChoices = [];
        const results = await PeliasClient.autocomplete(searchText, focus);
        // TODO: handle execptions in fetch and multiple searches
        this.isLoading = false;
        if (!results.bbox) {
          console.error('search results missing bounding box');
        } else if (results.bbox.length != 4) {
          console.error('unexpected bbox dimensions');
        } else {
          map.fitBounds(results.bbox);
        }

        for (const feature of results.features) {
          if (!feature.properties?.gid) {
            console.error('feature was missing gid');
            continue;
          }
          let gid = feature.properties.gid;
          let id = PlaceId.gid(gid);
          places.push(Place.fromFeature(id, feature));
        }
      } catch (e) {
        console.warn('error with autocomplete', e);
      }

      this.placeChoices = places;
      this.renderPlacesOnMap();
    },
    renderPlacesOnMap() {
      const map = getBaseMap();
      if (!map) {
        console.error('basemap was unexpectedly unset');
        return;
      }

      map.removeAllMarkers();
      this.placeChoices.forEach((place: Place, idx: number) => {
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
  mounted(): void {
    this.updateSearch(this.searchText);
  },
  async unmounted(): Promise<void> {
    const map = await baseMapPromise;
    this.placeChoices = [];
    map.removeAllMarkers();
  },
});
</script>
