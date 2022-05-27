<template>
  <div>
    <q-input
      ref="autoCompleteInput"
      class="main-search-bar"
      :label="$props.hint ? $props.hint : 'Where to?'"
      v-model="inputText"
      :clearable="true"
      :input-style="{ color: 'black' }"
      :outlined="true"
      :debounce="0"
      v-on:clear="() => selectPoi(undefined)"
      v-on:blur="deferHide(castToMenu($refs.autoCompleteMenu))"
      v-on:beforeinput="
        (event) =>
          updateAutocompleteEventBeforeInput(
            event,
            castToMenu($refs.autoCompleteMenu)
          )
      "
      v-on:update:model-value="
        () =>
          updateAutocompleteEventRawString(castToMenu($refs.autoCompleteMenu))
      "
    >
    </q-input>
    <q-menu
      auto-close
      ref="autoCompleteMenu"
      :no-focus="true"
      :no-refocus="true"
      :target="castToTarget($refs.autoCompleteInput)"
    >
      <q-item
        :key="item?.key"
        v-for="item in autocompleteOptions"
        clickable
        v-on:click="() => selectPoi(item)"
        v-on:mouseenter="() => hoverPoi(item)"
        v-on:mouseleave="() => hoverPoi(undefined)"
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
    </q-menu>
  </div>
</template>

<script lang="ts">
import { defineComponent, Ref, ref } from 'vue';
import {
  localizeAddress,
  LongLat,
  POI,
  poiDisplayName,
} from 'src/components/models';
import { Event, Marker } from 'maplibre-gl';
import { map } from './BaseMap.vue';
import { QMenu } from 'quasar';

const isAndroid = /(android)/i.test(navigator.userAgent);

export default defineComponent({
  name: 'SearchBox',
  props: {
    forceText: String,
    hint: String,
  },
  methods: {},
  watch: {
    forceText: {
      immediate: true,
      deep: true,
      handler(newVal) {
        this.inputText = newVal;
      },
    },
  },
  emits: ['update:modelValue'],
  data: function () {
    return {
      poi: this.poiSelected,
    };
  },
  unmounted: function () {
    this.onUnmounted();
  },
  beforeUnmount: function () {
    this.onUnmounted();
  },
  setup: function (props, context) {
    const inputText = ref('');
    const poiSelected: Ref<POI | undefined> = ref(undefined);
    const poiHovered: Ref<POI | undefined> = ref(undefined);
    const autocompleteOptions: Ref<(POI | undefined)[]> = ref([]);

    var hoverMarker: Marker | undefined = undefined;

    const updateAutocomplete = async function (
      currentTextValue: string,
      target?: HTMLInputElement
    ) {
      const value = target ? target.value : currentTextValue;
      const response = await fetch(
        `/photon/api?q=${encodeURIComponent(value)}&limit=10`
      );
      if (response.status != 200) {
        autocompleteOptions.value = [];
        return;
      }
      const results = await response.json();
      var options: POI[] = [];
      for (const feature of results.features) {
        var address = localizeAddress(
          feature.properties.housenumber,
          feature.properties.street,
          feature.properties.locality,
          feature.properties.city
        );

        const coordinates = feature?.geometry?.coordinates;
        const position: LongLat | undefined = coordinates
          ? { long: coordinates[0], lat: coordinates[1] }
          : undefined;
        options.push({
          name: feature.properties.name,
          address: address,
          key: feature.properties.osm_id,
          position: position,
          id: feature?.properties?.osm_id,
          type: feature?.properties?.osm_type,
        });
      }
      autocompleteOptions.value = options;
    };
    return {
      inputText,
      autocompleteOptions,
      poiSelected,
      poiHovered,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      castToTarget(target: any) {
        return target as Element;
      },
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      castToMenu(menu: any) {
        return menu as QMenu;
      },
      deferHide(menu: QMenu) {
        setTimeout(() => menu.hide(), 500);
      },
      updateAutocompleteEventRawString(menu: QMenu) {
        menu.show();
        if (poiSelected.value) {
          poiSelected.value = undefined;
        }
        if (poiHovered.value) {
          poiHovered.value = undefined;
        }
        if (!isAndroid) {
          setTimeout(() => updateAutocomplete(inputText.value));
        }
      },
      updateAutocompleteEventBeforeInput(event: Event, menu: QMenu) {
        const inputEvent = event as InputEvent;
        menu.show();
        if (poiSelected.value) {
          poiSelected.value = undefined;
        }
        if (poiHovered.value) {
          poiHovered.value = undefined;
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
      selectPoi(poi: POI | undefined) {
        poiSelected.value = poi;
        if (poi) {
          inputText.value = poiDisplayName(poi);
        } else {
          inputText.value = '';
        }
        if (hoverMarker) {
          hoverMarker.remove();
        }
        context.emit('update:modelValue', poi);
      },
      hoverPoi(poi: POI | undefined) {
        poiHovered.value = poi;

        if (hoverMarker) {
          hoverMarker.remove();
        }
        if (map && poi?.position?.long && poi?.position?.lat) {
          hoverMarker = new Marker({ color: '#11111155' }).setLngLat([
            poi?.position?.long,
            poi?.position?.lat,
          ]);
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
</script>
