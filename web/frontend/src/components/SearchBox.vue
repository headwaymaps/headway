<template>
  <div class="search-box">
    <q-select
      ref="selectField"
      use-input
      behavior="menu"
      hide-dropdown-icon
      outlined
      dense
      fill-input
      hide-selected
      input-class="search-box-input"
      clearable
      :label="$props.hint || $t('where_to_question')"
      :readonly="readonly"
      :model-value="selectedPlace"
      @update:model-value="selectPlace"
      @filter="onFilter"
      :options="placeChoices"
      :option-label="(place: Place) => place.name ?? place.address ?? ''"
      v-on:keydown="onKeyDown"
      v-on:blur="onBlur"
      input-debounce="0"
    >
      <template #option="{ opt, selected, itemProps }">
        <q-item
          v-bind="itemProps"
          v-on:mouseenter="onHoverPlace(opt)"
          v-on:mouseleave="onHoverPlace(undefined)"
          :style="selected ? 'border-left: solid black 2px;' : ''"
        >
          <q-item-section>
            <q-item-label>{{ opt.name ?? opt.address }}</q-item-label>
            <q-item-label v-if="opt.name" caption>{{
              opt.address
            }}</q-item-label>
          </q-item-section>
        </q-item>
      </template>
    </q-select>
  </div>
</template>

<style lang="scss">
.search-box {
  background: white;
  border-radius: 4px;
}
</style>

<script lang="ts">
import { defineComponent, PropType, Ref, ref } from 'vue';
import { throttle } from 'lodash';
import { Marker } from 'maplibre-gl';
import { map } from './BaseMap.vue';
import { Platform, QSelect } from 'quasar';
import Place, { PlaceId } from 'src/models/Place';
import PeliasClient from 'src/services/PeliasClient';
import Markers from 'src/utils/Markers';
import { supportsHover } from 'src/utils/misc';

export default defineComponent({
  name: 'SearchBox',
  props: {
    hint: String,
    readonly: Boolean,
    resultsCallback: Function as PropType<(results?: Place[]) => void>,
    forcePlace: Place,
    initialInputText: String,
  },
  data(): {
    selectedPlace?: Place;
  } {
    return { selectedPlace: this.forcePlace };
  },
  methods: {
    async onFilter(
      inputText: string,
      doneFn: (callbackFn: () => void, afterFn?: (ref: QSelect) => void) => void
    ) {
      // HACK: we call the `doneFn` immediately, otherwise the search results keep
      // flickering while the user types out their query.
      doneFn(() => {
        /* I'm not sure why this method is required.. I don't need to do anything further */
      });
      await this.updateAutocomplete();
    },
    onKeyDown(event: KeyboardEvent): void {
      if (event.key == 'Enter') {
        let searchText = this.currentSearchText();
        if (searchText) {
          this.$emit('didSubmitSearch', searchText);
        }
      }
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
    inputTextDidChange() {
      this.removeHoverMarkers();
      this.updateAutocomplete();
    },
    removeHoverMarkers() {
      if (this.hoverMarker) {
        this.hoverMarker.remove();
        this.hoverMarker = undefined;
      }
    },
    selectPlace(place?: Place) {
      this.selectedPlace = place;
      this.$emit('didSelectPlace', place);
      this.removeHoverMarkers();
    },
    onHoverPlace(place?: Place) {
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
      this.placeHovered = place;
      this.removeHoverMarkers();

      if (!map) {
        console.error('map was unexpectedly unset');
        return;
      }

      if (place) {
        this.hoverMarker = Markers.inactive().setLngLat(place.point);
        this.hoverMarker.addTo(map);
      }
    },
  },
  watch: {
    forcePlace: {
      handler(newValue?: Place) {
        this.selectedPlace = newValue;
      },
    },
  },
  emits: ['didSelectPlace', 'didSubmitSearch'],
  mounted(): void {
    if (this.initialInputText) {
      this.selectField?.updateInputValue(this.initialInputText);
    }
  },
  unmounted(): void {
    console.assert(!this.unmounted, 'should only unmount once');
    this.unmounted = true;
    this.removeHoverMarkers();
  },
  setup() {
    const placeHovered: Ref<Place | undefined> = ref(undefined);
    const placeChoices: Ref<Place[] | undefined> = ref(undefined);
    const hoverMarker: Ref<Marker | undefined> = ref(undefined);
    const selectField: Ref<QSelect | null> = ref(null);
    const mostRecentlyCompletedSearchText: Ref<string> = ref('');
    const unmounted: Ref<boolean> = ref(false);

    function currentSearchText(): string {
      // This is a dirty probably brittle hack.
      //
      // AFAICT there's no way to get the *current* text in the input field. All
      // the available "blessed" ways rely on this value propogating via async
      // callbacks (e.g. @input-value or via @filter), but we want to handle the
      // "Enter" key (on keydown) which happens before those other callbacks
      // are called. Thus we need to punch through the abstractions and just grab
      // the value directly from the input element.

      //let inputEl: HTMLInputElement = $('input.search-box-input', this.selectField().$el);
      let inputEl: HTMLInputElement = selectField.value?.$el.querySelector(
        'input.search-box-input'
      );
      console.assert(
        inputEl,
        'expected to find input element within search box'
      );
      return inputEl.value;
    }

    async function _updateAutocomplete(): Promise<void> {
      let searchText = currentSearchText();
      if (searchText.length == 0) {
        mostRecentlyCompletedSearchText.value = '';
        placeChoices.value = undefined;
        return;
      }

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
      //
      // We *do* want to update autocomplete as the user extends a query.
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
      if (unmounted.value || !currentSearchText().includes(searchText)) {
        // discarding old results
        return;
      }
      mostRecentlyCompletedSearchText.value = searchText;
      placeChoices.value = places;
    }
    const throttleMs = 200;
    const updateAutocomplete = throttle(_updateAutocomplete, throttleMs, {
      trailing: true,
    });

    return {
      hoverMarker,
      selectField,
      updateAutocomplete,
      placeChoices,
      placeHovered,
      currentSearchText,
      unmounted,
    };
  },
});
</script>
