<template>
  <div class="search-box">
    <q-input
      ref="autoCompleteInput"
      :label="$props.hint ? $props.hint : $t('where_to_question')"
      v-model="inputText"
      clearable
      :readonly="readonly"
      :input-style="{ color: 'black' }"
      :outlined="true"
      :debounce="0"
      :dense="true"
      v-on:clear="selectPlace(undefined)"
      v-on:blur="onBlur"
      v-on:beforeinput="
  (event: Event) =>
    updateAutocompleteEventBeforeInput(
      event,
      autoCompleteMenu()
      )
      "
      v-on:update:model-value="
        () => updateAutocompleteEventRawString(autoCompleteMenu())
      "
    >
    </q-input>
    <q-menu
      auto-close
      ref="autoCompleteMenu"
      :no-focus="true"
      :no-refocus="true"
      v-on:before-hide="removeHoverMarkers"
      :target="($refs.autoCompleteInput as Element)"
    >
      <q-list>
        <q-item
          :key="item?.serializedId()"
          v-for="item in autocompleteOptions"
          clickable
          v-on:click="selectPlace(item)"
          v-on:mouseenter="hoverPlace(item)"
          v-on:mouseleave="hoverPlace(undefined)"
        >
          <q-item-section>
            <q-item-label>{{
              item?.name ? item.name : item?.address
            }}</q-item-label>
            <q-item-label v-if="item?.name" caption>{{
              item.address
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
import { defineComponent, Ref, ref } from 'vue';
import { throttle } from 'lodash';
import { Event, Marker } from 'maplibre-gl';
import { map } from './BaseMap.vue';
import { QMenu, Platform } from 'quasar';
import Place, { PlaceId } from 'src/models/Place';

const isAndroid = /(android)/i.test(navigator.userAgent);

export default defineComponent({
  name: 'SearchBox',
  props: {
    forceText: String,
    hint: String,
    readonly: Boolean,
  },
  methods: {
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
  },
  watch: {
    forceText: {
      immediate: true,
      deep: true,
      handler(newVal) {
        this.inputText = newVal;
      },
    },
  },
  emits: ['didSelectPlace'],
  unmounted: function () {
    this.onUnmounted();
  },
  beforeUnmount: function () {
    this.onUnmounted();
  },
  setup: function (props, ctx) {
    const inputText = ref('');
    const placeHovered: Ref<Place | undefined> = ref(undefined);
    const autocompleteOptions: Ref<Place[] | undefined> = ref([]);
    let requestIdx = 0;
    let mostRecentResultsRequestIdx = 0;

    var hoverMarker: Marker | undefined = undefined;

    const _updateAutocomplete = async function (
      currentTextValue: string,
      target?: HTMLInputElement
    ) {
      const value = target ? target.value : currentTextValue;
      if (!value) {
        return;
      }
      let url = undefined;
      if (map && map.getZoom() > 6) {
        const mapCenter = map?.getCenter();
        url = `/pelias/v1/autocomplete?text=${encodeURIComponent(
          value
        )}&focus.point.lon=${mapCenter?.lng}&focus.point.lat=${mapCenter?.lat}`;
      } else {
        url = `/pelias/v1/autocomplete?text=${encodeURIComponent(value)}`;
      }
      const thisRequestIdx = requestIdx;
      requestIdx++;
      const response = await fetch(url);
      if (response.status != 200) {
        if (thisRequestIdx > mostRecentResultsRequestIdx) {
          // Don't clobber existing good results with an error from a stale request
          autocompleteOptions.value = [];
        }
        return;
      }
      if (thisRequestIdx < mostRecentResultsRequestIdx) {
        // not updating autocomplete for a stale req
        return;
      }
      mostRecentResultsRequestIdx = thisRequestIdx;

      const results = await response.json();
      var options: Place[] = [];
      for (const feature of results.features) {
        // TODO: Not sure if this ever happens.
        console.assert(feature.properties.gid);
        let gid = feature.properties.gid;
        let id = PlaceId.gid(gid);
        options.push(Place.fromFeature(id, feature));
      }
      autocompleteOptions.value = options;
    };
    const throttleMs = 200;
    const updateAutocomplete = throttle(_updateAutocomplete, throttleMs, {
      trailing: true,
    });

    return {
      inputText,
      autocompleteOptions,
      placeHovered,
      deferHide(menu: QMenu) {
        setTimeout(() => {
          menu.hide();
          if (hoverMarker) {
            hoverMarker.remove();
            hoverMarker = undefined;
          }
        }, 500);
      },
      removeHoverMarkers() {
        if (hoverMarker) {
          hoverMarker.remove();
          hoverMarker = undefined;
        }
      },
      updateAutocompleteEventRawString(menu: QMenu) {
        menu.show();
        if (placeHovered.value) {
          placeHovered.value = undefined;
        }
        if (!isAndroid) {
          setTimeout(() => updateAutocomplete(inputText.value));
        }
      },
      updateAutocompleteEventBeforeInput(event: Event, menu: QMenu) {
        const inputEvent = event as InputEvent;
        menu.show();
        if (placeHovered.value) {
          placeHovered.value = undefined;
        }
        if (isAndroid) {
          setTimeout(() =>
            updateAutocomplete(
              inputText.value,
              inputEvent.target as HTMLInputElement
            )
          );
        }
      },
      selectPlace(place?: Place) {
        ctx.emit('didSelectPlace', place);
        setTimeout(() => {
          if (hoverMarker) hoverMarker.remove();
        });
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

        if (hoverMarker) {
          hoverMarker.remove();
        }

        if (!map) {
          console.error('map was unexpectedly unset');
          return;
        }

        if (place) {
          hoverMarker = new Marker({ color: '#11111155' }).setLngLat(
            place.point
          );
          hoverMarker.addTo(map);
        }
      },
      onUnmounted() {
        if (hoverMarker) {
          hoverMarker.remove();
        }
      },
    };
  },
});

function supportsHover(): boolean {
  return window.matchMedia('(hover: hover)').matches;
}
</script>
