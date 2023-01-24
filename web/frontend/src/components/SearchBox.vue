<template>
  <div class="search-box">
    <q-input
      ref="inputField"
      :label="$props.hint || $t('where_to_question')"
      v-model="inputText"
      clearable
      :readonly="readonly"
      :input-style="{ color: 'black' }"
      :outlined="true"
      :dense="true"
      v-on:clear="selectPlace(undefined)"
      v-on:keydown="onKeyDown"
      v-on:blur="onBlur"
      v-on:update:model-value="inputTextDidChange"
    />
    <q-menu
      auto-close
      ref="autocompleteMenu"
      :no-focus="true"
      :no-refocus="true"
      v-on:before-hide="removeHoverMarkers"
      :target="($refs.inputField as Element)"
      v-if="!resultsCallback"
    >
      <q-list>
        <q-item
          :key="place.serializedId()"
          v-for="place in placeChoices"
          clickable
          v-on:click="selectPlace(place)"
          v-on:mouseenter="onHoverPlace(place)"
          v-on:mouseleave="onHoverPlace(undefined)"
        >
          <q-item-section>
            <q-item-label>{{ place.name ?? place.address }}</q-item-label>
            <q-item-label v-if="place.name" caption>{{
              place.address
            }}</q-item-label>
          </q-item-section>
        </q-item>
      </q-list>
    </q-menu>
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
import { QInput, QMenu, Platform } from 'quasar';
import Place, { PlaceId } from 'src/models/Place';
import PeliasClient from 'src/services/PeliasClient';
import Markers from 'src/utils/Markers';
import { supportsHover } from 'src/utils/misc';

export default defineComponent({
  name: 'SearchBox',
  props: {
    forceText: String,
    hint: String,
    readonly: Boolean,
    resultsCallback: Function as PropType<(results?: Place[]) => void>,
  },
  methods: {
    autocompleteMenu(): QMenu {
      return this.$refs.autocompleteMenu as QMenu;
    },
    inputField(): QInput {
      return this.$refs.inputField as QInput;
    },
    onKeyDown(event: KeyboardEvent): void {
      if (event.key == 'Enter') {
        let searchText = this.inputField().modelValue;
        if (searchText) {
          this.$emit('didSubmitSearch', searchText.toString());
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
    forceText: {
      immediate: true,
      deep: true,
      handler(newVal?: string) {
        this.inputText = newVal;
      },
    },
    placeChoices: {
      handler(newVal?: Place[]) {
        this.resultsCallback?.(newVal);
      },
    },
  },
  emits: ['didSelectPlace', 'didSubmitSearch'],
  unmounted(): void {
    this.removeHoverMarkers();
  },
  setup() {
    const inputText: Ref<string | undefined> = ref(undefined);
    const placeHovered: Ref<Place | undefined> = ref(undefined);
    const placeChoices: Ref<Place[] | undefined> = ref([]);
    var hoverMarker: Ref<Marker | undefined> = ref(undefined);

    async function _updateAutocomplete(): Promise<void> {
      if (!inputText.value) {
        placeChoices.value = undefined;
        return;
      }

      let searchText = inputText.value.trim();
      if (searchText.length == 0) {
        placeChoices.value = undefined;
        return;
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

      // We want to update autocomplete as the user extends a query.
      // But we don't want to show a no longer relevant, e.g. if the user deleted or edited characters.
      //
      // request text: "Se",   current inputField: "Sea",  <-- show stale request results, the user is still typing out the word
      // request text: "Sea",  current inputField: "Seatt" <-- show stale request results, the user is still typing out the word
      // request text: "Seat", current inputField: "Sea",  <-- discard stale request results, the user has deleted part of that previous query
      // request text: "S",    current inputField: "",     <-- discard stale request results, the user has deleted the last letter of the query
      if (!inputText.value.trim().includes(searchText)) {
        // discarding old results
        return;
      }

      placeChoices.value = places;
    }
    const throttleMs = 200;
    const updateAutocomplete = throttle(_updateAutocomplete, throttleMs, {
      trailing: true,
    });

    return {
      inputText,
      hoverMarker,
      updateAutocomplete,
      placeChoices,
      placeHovered,
    };
  },
});
</script>
