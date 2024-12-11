<template>
  <div class="search-box">
    <div class="input-field">
      <input
        ref="autoCompleteInput"
        :tabindex="tabindex ?? 0"
        :placeholder="$props.hint || $t('where_to_question')"
        :value="inputText"
        :readonly="readonly"
        @blur="onBlur"
        @input="onInput"
        @keydown="onKeyDown"
      />
      <q-btn
        round
        dense
        unelevated
        padding="0"
        style="margin-right: 8px"
        class="clear-button"
        icon="cancel"
        color="transparent"
        text-color="grey"
        @click="clear()"
      />
      <slot></slot>
    </div>
    <div
      ref="autoCompleteMenu"
      class="auto-complete-menu"
      @before-hide="removeHoverMarkers"
    >
      <q-list>
        <q-item
          v-for="(place, index) in placeChoices"
          :key="place.serializedId()"
          clickable
          :class="index == highlightedIndex ? 'highlighted' : ''"
          @click="selectPlace(place)"
          @mouseenter="hoverPlace(place)"
          @mouseleave="hoverPlace(undefined)"
        >
          <q-item-section>
            <q-item-label>{{ place.name || place.address }}</q-item-label>
            <q-item-label v-if="place.name" caption>{{
              place.address
            }}</q-item-label>
          </q-item-section>
        </q-item>
      </q-list>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, Ref, ref } from 'vue';
import { throttle } from 'lodash';
import { Marker } from 'maplibre-gl';
import { map } from './BaseMap.vue';
import { Platform } from 'quasar';
import Place, { PlaceId } from 'src/models/Place';
import PeliasClient from 'src/services/PeliasClient';
import Markers from 'src/utils/Markers';
import { supportsHover } from 'src/utils/misc';
import { placeDisplayName } from 'src/i18n/utils';

export default defineComponent({
  name: 'SearchBox',
  props: {
    tabindex: {
      type: Number,
      default: 0,
    },
    initialInputText: {
      type: String,
      default: undefined,
    },
    initialPlace: {
      type: Place,
      default: undefined,
    },
    hint: {
      type: String,
      default: undefined,
    },
    readonly: Boolean,
  },
  emits: ['didSelectPlace', 'didSubmitSearch'],
  setup: function (props, ctx) {
    const inputText: Ref<string | undefined> = ref(
      props.initialInputText ||
        (props.initialPlace ? placeDisplayName(props.initialPlace) : undefined),
    );
    const placeHovered: Ref<Place | undefined> = ref(undefined);
    const highlightedIndex: Ref<number | undefined> = ref(undefined);
    const placeChoices: Ref<Place[]> = ref([]);
    const mostRecentSearchIdx = ref(0);
    const isUnmounted = ref(false);
    let hoverMarker: Marker | undefined = undefined;

    type Query = { text: string; idx: number };

    let searchIdx = 0;
    let mostRecentlyCompletedQuery = { text: '', idx: 0 };

    const _updatePlaceChoices = async function () {
      searchIdx++;
      const query: Query = { text: inputText.value ?? '', idx: searchIdx };
      if (query.text.length == 0) {
        mostRecentlyCompletedQuery = query;
        placeChoices.value = [];
        return;
      }

      // Note: this Idx is for a *search* not for an autocomplete. We want
      // to skip any autocompletes UI if the user has subsequently submitted a
      // full blown search.
      const thisSearchIdx = mostRecentSearchIdx.value;

      // Keep the autocomplete menu up only if it's still relevant.
      if (query.text.includes(mostRecentlyCompletedQuery.text)) {
        // console.debug('keeping old results while appending to input field');
      } else if (mostRecentlyCompletedQuery.text.includes(query.text)) {
        // console.debug('keeping old results while deleting characters from input field');
      } else {
        // console.debug('immediately clearing old results since the input text is no longer relvant');
        placeChoices.value = [];
      }

      let focus = undefined;
      if (map && map.getZoom() > 6) {
        focus = map.getCenter();
      }

      const places: Place[] = [];
      try {
        const results = await PeliasClient.autocomplete(query.text, focus);
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

      // We have `awaited`, and need to make sure it makes sense to proceed...

      // Firstly - Quit if the user has left the page.
      if (isUnmounted.value) {
        // console.debug('isUnmounted');
        return;
      }

      // Next, cancel the autocomplete if the user has pressed Enter to search
      // in the meanwhile so we don't pop up the autocomplete menu over their
      // search results.
      if (mostRecentSearchIdx.value > thisSearchIdx) {
        // console.debug(`mostRecentSearchIdx.value > thisSearchIdx (${mostRecentSearchIdx.value} > ${thisSearchIdx})`);
        return;
      }

      // Finally - update the results as long as they are more recent.
      // It is *common* to receive results out of order - short queries take
      // longer since they have more matches.
      if (query.idx < mostRecentlyCompletedQuery.idx) {
        // console.debug(`discarding irrelevant results. inputText: ${inputText.value}`, query);
        return;
      }

      // console.debug('completed query', query)
      mostRecentlyCompletedQuery = query;
      placeChoices.value = places;
    };
    const throttleMs = 200;
    const updatePlaceChoices = throttle(_updatePlaceChoices, throttleMs, {
      trailing: true,
    });

    function removeHoverMarkers() {
      if (hoverMarker) {
        hoverMarker.remove();
        hoverMarker = undefined;
      }
    }

    return {
      inputText,
      placeChoices,
      placeHovered,
      highlightedIndex,
      mostRecentSearchIdx,
      removeHoverMarkers,
      updateAutocomplete() {
        if (placeHovered.value) {
          placeHovered.value = undefined;
        }
        highlightedIndex.value = undefined;
        updatePlaceChoices();
      },
      selectPlace(place?: Place) {
        ctx.emit('didSelectPlace', place);
        removeHoverMarkers();
        // dismiss menu when a place is selected
        if (place) {
          const el = document.activeElement as HTMLElement;
          el.blur();
        }
      },
      hoverPlace(place?: Place) {
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
        placeHovered.value = place;

        removeHoverMarkers();

        if (!map) {
          console.error('map was unexpectedly unset');
          return;
        }

        if (place) {
          hoverMarker = Markers.inactive().setLngLat(place.point);
          hoverMarker.addTo(map);
        }
      },
      unmounted() {
        removeHoverMarkers();
        isUnmounted.value = true;
      },
    };
  },
  data(): {
    isAndroid: boolean;
  } {
    const isAndroid = /(android)/i.test(navigator.userAgent);
    return { isAndroid };
  },
  watch: {
    initialPlace: {
      handler(newValue?: Place, oldValue?: Place) {
        if (newValue != oldValue) {
          this.placeChoices = [];
        }

        this.inputText = newValue ? placeDisplayName(newValue) : undefined;
      },
    },
  },
  methods: {
    highlightNext(): void {
      if (this.placeChoices.length == 0) {
        this.highlightedIndex = undefined;
        return;
      }

      if (this.highlightedIndex === undefined) {
        this.highlightedIndex = 0;
      } else {
        this.highlightedIndex =
          (this.highlightedIndex + 1) % this.placeChoices.length;
      }
    },
    highlightPrevious(): void {
      if (this.placeChoices.length == 0) {
        this.highlightedIndex = undefined;
        return;
      }

      if (this.highlightedIndex === undefined) {
        this.highlightedIndex = this.placeChoices.length - 1;
      } else if (this.highlightedIndex == 0) {
        this.highlightedIndex = this.placeChoices.length - 1;
      } else {
        this.highlightedIndex = this.highlightedIndex - 1;
      }
    },
    onKeyDown(event: KeyboardEvent): void {
      if (event.key == 'Enter') {
        if (this.highlightedIndex != undefined) {
          const place = this.placeChoices[this.highlightedIndex];
          if (!place) {
            console.assert(false, 'missing place for highlightedIndex');
            return;
          }
          this.selectPlace(place);
          return;
        }
        this.mostRecentSearchIdx++;
        const searchText = this.inputText;
        // If the user hit enter, don't pop autocomplete results over the search
        // results.
        this.autoCompleteInput().blur();
        if (searchText) {
          this.$emit('didSubmitSearch', searchText);
        }
      } else if (event.key == 'ArrowDown') {
        this.highlightNext();
        event.preventDefault();
      } else if (event.key == 'ArrowUp') {
        this.highlightPrevious();
        event.preventDefault();
      }
    },
    onInput(): void {
      this.inputText = this.autoCompleteInput().value;
      this.updateAutocomplete();
    },
    onBlur(): void {
      if (Platform.is.ios) {
        // iOS (on at least 16.1) "helpfully" moves the focused input towards
        // the middle of the screen, but because out input is in a fixed header
        // at the top of our app, this has the affect of adding a bunch of
        // padding (~100px) at the top of our app, even after the keyboard is
        // dismissed.
        //
        // I only duplicated this on a physical iPhone SE 2018 16.1. It went
        // away after updating to 16.2, so if this work-around causes problems,
        // we can delete it some day as the browser share declines.
        //
        // I don't have a physical iPhoneX style device, and couldn't induce this
        // behavior on the simulator. I'm not sure if that's because it doesn't
        // affect that layout, or because it doesn't affect the simulator.
        //
        // NOTE: scrolling to 0,0 doesn't seem to do anything. Inspecting
        // `window.scrollY` is 0 before *and* after this scroll, so maybe the
        // browser thinks it's a no-op.
        window.scroll(0, -1);
      }
    },
    autoCompleteMenu(): HTMLElement {
      return this.$refs.autoCompleteMenu as HTMLElement;
    },
    autoCompleteInput(): HTMLInputElement {
      return this.$refs.autoCompleteInput as HTMLInputElement;
    },
    clear(): void {
      this.inputText = '';
      this.selectPlace(undefined);
      this.placeChoices = [];
      this.autoCompleteInput().focus();
    },
  },
});
</script>

<style lang="scss">
.search-box {
  position: relative;

  background-color: white;

  box-shadow: 0 0 2px 1px #666;
  border-radius: 4px;

  .auto-complete-menu {
    display: none;
    position: absolute;
    width: 100%;
    max-height: 80vh;

    background-color: white;
    overflow-y: scroll;
    border-top: none;
    border-radius: 0 0 4px 4px;

    // note the shadow is "brighter" than the shadow around the input text.
    // I'm not sure why this is required, but it matches better this way.
    // (tested on Safari and Chrome on macos)
    box-shadow: 0 0 3px 2px #333;

    // prevent box shadow from casting "up" onto tex field
    clip-path: inset(0 -4px -4px -4px);

    .q-item {
      padding-left: 8px;
      padding-right: 8px;
    }

    .q-item.highlighted {
      background-color: #ededed;
    }

    z-index: 1;
  }

  &:focus-within {
    box-shadow: 0 0 3px 2px #222;

    &:has(.auto-complete-menu .q-item:first-child) {
      border-bottom-left-radius: 0;
      border-bottom-right-radius: 0;
    }

    &:has(.auto-complete-menu .q-item:first-child) .input-field {
      border-bottom: solid #ddd 1px;
    }

    .auto-complete-menu:has(.q-item) {
      display: block;
    }
  }

  &:has(input[readonly]) {
    box-shadow: none;
    border: dashed #aaa 1px;
  }
  .input-field {
    font-size: 16px;
    display: flex;
    height: 100%;
    flex-direction: row;
    align-items: center;
    input {
      flex: 1;
      border: none;
      background-color: transparent;
    }

    input:focus {
      outline: none;
    }

    input,
    .clear-button {
      padding: 4px 8px;
    }

    // only show clear-button when input has content
    // and is editable
    &:has(input:placeholder-shown) .clear-button,
    &:has(input[readonly]) .clear-button {
      display: none;
    }

    // We don't want the settings link to interfere with the search field contents
    // So hide the settings button when the user has, or is about to, give input
    &:has(input:focus) .settings-button,
    &:not(:has(input:placeholder-shown)) .settings-button {
      display: none;
    }
  }
}
</style>
