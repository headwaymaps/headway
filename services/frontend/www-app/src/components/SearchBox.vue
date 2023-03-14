<template>
  <div class="search-box">
    <div class="input-field">
      <input
        ref="autoCompleteInput"
        :placeholder="$props.hint || $t('where_to_question')"
        :value="inputText"
        clearable
        :readonly="readonly"
        :debounce="0"
        :dense="true"
        type="search"
        @blur="onBlur"
        @input="onInput"
        @keydown="onKeyDown"
      />
      <q-btn
        round
        dense
        unelevated
        padding="0"
        class="clear-button"
        icon="cancel"
        color="transparent"
        text-color="grey"
        @click="clear()"
      />
    </div>
    <div
      ref="autoCompleteMenu"
      class="auto-complete-menu"
      v-on:before-hide="removeHoverMarkers"
      :hidden="!(placeChoices && placeChoices.length > 0)"
    >
      <q-list>
        <q-item
          :key="place.serializedId()"
          v-for="place in placeChoices"
          clickable
          v-on:click="selectPlace(place)"
          v-on:mouseenter="hoverPlace(place)"
          v-on:mouseleave="hoverPlace(undefined)"
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

<style lang="scss">
.search-box {
  position: relative;

  background-color: white;
  border: solid #aaa 1px;
  border-radius: 4px;

  .auto-complete-menu {
    display: none;
    position: absolute;
    // safari
    width: calc(100% + 3px);
    left: -1.5px;
    // chrome
    width: calc(100% + 2px);
    left: -1px;

    background-color: white;
    border: solid #aaa 1px;
    border-top: none;
    border-radius: 0 0 4px 4px;

    // note the shadow is "brighter" than the shadow around the input text.
    // I'm not sure why this is required, but it matches better this way.
    // (tested on Safari and Chrome on macos)
    box-shadow: 0 0 2px 1px #ccc;

    // prevent box shadow from casting "up" onto tex field
    clip-path: inset(0 -4px -4px -4px);

    .q-item {
      padding-left: 8px;
      padding-right: 8px;
    }

    z-index: 1;

    .q-item:first-child {
      border-top: solid #aaa 1px;
    }
  }

  &:focus-within {
    box-shadow: 0 0 3px 1px #aaa;

    &:has(.auto-complete-menu .q-item:first-child) {
      border-bottom-left-radius: 0;
      border-bottom-right-radius: 0;
    }

    .auto-complete-menu:has(.q-item) {
      display: block;
    }
  }

  .input-field {
    padding: 4px 8px;
    display: flex;
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
  }
}
</style>

<script lang="ts">
import { defineComponent, Ref, ref } from 'vue';
import { throttle } from 'lodash';
import { Marker } from 'maplibre-gl';
import { map } from './BaseMap.vue';
import { QMenu, Platform, QInput } from 'quasar';
import Place, { PlaceId } from 'src/models/Place';
import PeliasClient from 'src/services/PeliasClient';
import Markers from 'src/utils/Markers';
import { supportsHover } from 'src/utils/misc';
import { placeDisplayName } from 'src/i18n/utils';

export default defineComponent({
  name: 'SearchBox',
  props: {
    initialInputText: String,
    initialPlace: Place,
    hint: String,
    readonly: Boolean,
  },
  data(): {
    isAndroid: boolean;
    menuShowing: boolean;
  } {
    const isAndroid = /(android)/i.test(navigator.userAgent);
    return { isAndroid, menuShowing: false };
  },
  methods: {
    onKeyDown(event: KeyboardEvent): void {
      if (event.key == 'Enter') {
        this.mostRecentSearchIdx++;
        let searchText = this.inputText;
        if (searchText) {
          this.$emit('didSubmitSearch', searchText);
        }
      }
    },
    onInput(): void {
      this.inputText = this.autoCompleteInput().value;
      this.updateAutocomplete(this.autoCompleteMenu());
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
    autoCompleteMenu(): QMenu {
      return this.$refs.autoCompleteMenu as QMenu;
    },
    autoCompleteInput(): HTMLInputElement {
      return this.$refs.autoCompleteInput as HTMLInputElement;
    },
    clear(): void {
      this.inputText = '';
      this.selectPlace(undefined);
      this.placeChoices = undefined;
      this.autoCompleteInput().focus();
    },
  },
  watch: {
    initialPlace: {
      handler(newValue?: Place) {
        this.inputText = newValue ? placeDisplayName(newValue) : undefined;
      },
    },
  },
  emits: ['didSelectPlace', 'didSubmitSearch'],
  setup: function (props, ctx) {
    const inputText: Ref<string | undefined> = ref(
      props.initialInputText ||
        (props.initialPlace ? placeDisplayName(props.initialPlace) : undefined)
    );
    const placeHovered: Ref<Place | undefined> = ref(undefined);
    const placeChoices: Ref<Place[] | undefined> = ref([]);
    const mostRecentSearchIdx = ref(0);
    const mostRecentlyCompletedSearchText: Ref<string> = ref('');
    const isUnmounted = ref(false);
    let hoverMarker: Marker | undefined = undefined;

    const _updatePlaceChoices = async function () {
      const searchText = inputText.value ?? '';
      if (searchText.length == 0) {
        mostRecentlyCompletedSearchText.value = '';
        placeChoices.value = undefined;
        return;
      }

      // Note: this Idx is for a *search* not for an autocomplete. We want
      // to skip any autocompletes UI if the user has subsequently submitted a
      // full blown search.
      const thisSearchIdx = mostRecentSearchIdx.value;

      // If we're continuing to type out our search, keep the old results
      // up while we find the new ones - else, we clear the stale results here.
      if (searchText.includes(mostRecentlyCompletedSearchText.value)) {
        // console.debug('keeping old results while adding to input text');
      } else {
        // console.debug('immediately clearing old results since the input text is no longer relvant');
        mostRecentlyCompletedSearchText.value = '';
        placeChoices.value = undefined;
      }

      let focus = undefined;
      if (map && map.getZoom() > 6) {
        focus = map.getCenter();
      }

      let places: Place[] = [];
      try {
        const results = await PeliasClient.autocomplete(searchText, focus);
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

      // We have `awaited`, and need to make sure it makes sense to proceed...

      // Firstly - Quit if the user has left the page.
      if (isUnmounted.value) {
        return;
      }

      // Next, cancel the autocomplete if the user has pressed Enter to search
      // in the meanwhile so we don't pop up the autocomplete menu.
      if (mostRecentSearchIdx.value > thisSearchIdx) {
        return;
      }

      // Finally - we *do* want to update autocomplete as the user extends a query.
      //
      // But we don't want to show a no longer relevant result, e.g. if the user
      // deleted or edited characters, or if we are out-of-order: hearing back
      // from request #1 *after* we've already heard back from request #2. This
      // isn't a rare edge-case — often longer queries will return faster than
      // shorter prefix queries which have more matches.
      //
      // request text: "Se",   current inputField: "Sea",  <-- show stale request results, the user is still typing out the word
      // request text: "Sea",  current inputField: "Seatt" <-- show stale request results, the user is still typing out the word
      // request text: "Seat", current inputField: "Sea",  <-- discard stale request results, the user has deleted part of that previous query
      // request text: "S",    current inputField: "",     <-- discard stale request results, the user has deleted the last letter of the query
      if (!(inputText.value || '').includes(searchText)) {
        // discarding old results
        return;
      }

      mostRecentlyCompletedSearchText.value = searchText;
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
      mostRecentSearchIdx,
      removeHoverMarkers,
      updateAutocomplete(menu: QMenu) {
        console.log('updateAutocomplete. menu:', menu);
        if (placeHovered.value) {
          placeHovered.value = undefined;
        }
        updatePlaceChoices();
      },
      selectPlace(place?: Place) {
        ctx.emit('didSelectPlace', place);
        removeHoverMarkers();
      },
      hoverPlace(place?: Place) {
        if (!supportsHover()) {
          // FIX: selecting automcomplete item on mobile requires double
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
});
</script>
