<template>
  <div>
    <q-input
      ref="autoCompleteInput"
      id="autoCompleteInput"
      class="mainSearchBar"
      label="Search"
      label-color="white"
      v-model="inputText"
      :clearable="true"
      :input-style="{ color: 'white' }"
      :outlined="true"
      :debounce="0"
      v-on:clear="clearAutocomplete"
      v-on:beforeinput="updateAutocompleteEventBeforeInput"
      v-on:update:model-value="updateAutocompleteEventRawString"
    >
    </q-input>
    <q-menu
      persistent
      ref="autoCompleteMenu"
      :no-focus="true"
      :no-refocus="true"
      :target="inputField"
      v-show="menuShowing"
    >
      <q-item
        :key="item.key"
        v-for="item in autocompleteOptions"
        clickable
        v-on:click="() => updatePoi(item)"
        v-on:mouseenter="() => updateHoveredPoi(item)"
        v-on:mouseleave="() => updateHoveredPoi(undefined)"
      >
        <q-item-section>
          <q-item-label>{{
            item.name ? item.name : item.address
          }}</q-item-label>
          <q-item-label v-if="item.name" caption>{{
            item.address
          }}</q-item-label>
        </q-item-section>
      </q-item>
    </q-menu>
  </div>
</template>

<script lang="ts">
import { defineComponent, Ref, ref } from 'vue';
import { localizeAddress, LongLat, POI } from 'src/components/models';
import { Event } from 'maplibre-gl';
import { QMenu } from 'quasar';

const inputText = ref('');
const inputField = ref(undefined);
const menuShowing = ref(false);

async function updateAutocomplete(target?: HTMLInputElement) {
  const value = target ? target.value : inputText.value;
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

    // var name = feature.properties.name;
    // var caption = undefined;
    // if (name && address) {
    //   caption = address;
    // } else if (name) {
    //   caption = undefined;
    // } else if (address) {
    //   name = address;
    // } else {
    //   continue;
    // }
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
    });
  }
  autocompleteOptions.value = options;
}

const isAndroid = /(android)/i.test(navigator.userAgent);

const autocompleteOptions: Ref<POI[]> = ref([]);

var poi: POI | null | undefined = null;
var poiHovered: POI | null | undefined = null;
var autoCompleteMenu: typeof QMenu | null = null;

export default defineComponent({
  name: 'BaseMap',
  methods: {
    updateAutocompleteEventBeforeInput(event: Event) {
      const inputEvent = event as InputEvent;
      autoCompleteMenu?.show();
      if (null !== poi) {
        poi = null;
        this.$emit('poi_selected', poi);
      }
      if (isAndroid) {
        setTimeout(() =>
          updateAutocomplete(inputEvent.target as HTMLInputElement)
        );
      }
    },
    updateAutocompleteEventRawString() {
      autoCompleteMenu?.show();
      if (null !== poi) {
        poi = null;
        this.$emit('poi_selected', poi);
      }
      if (null !== poiHovered) {
        poiHovered = null;
        this.$emit('poi_hovered', poiHovered);
      }
      if (!isAndroid) {
        setTimeout(() => updateAutocomplete());
      }
    },
    setPoi(selectedPoi?: POI) {
      poi = selectedPoi;
      if (poi) {
        inputText.value = poi.name ? poi.name : (poi.address as string);
      } else {
        inputText.value = '';
      }
    },
    clearAutocomplete() {
      autoCompleteMenu?.hide();
      inputText.value = '';
      poi = null;
      this.$emit('poi_selected', poi);
      poiHovered = null;
      this.$emit('poi_selected', poiHovered);
    },
    updatePoi(item?: POI) {
      if (item !== poi) {
        poi = item;
        if (poi) {
          inputText.value = poi.name ? poi.name : (poi.address as string);
        } else {
          inputText.value = '';
        }
        autoCompleteMenu?.hide();
        this.$emit('poi_selected', poi);
      }
    },
    updateHoveredPoi(item?: POI) {
      if (item !== poiHovered) {
        poiHovered = item;
        this.$emit('poi_hovered', poiHovered);
      }
    },
  },
  emits: ['poi_selected', 'poi_hovered'],
  mounted: function () {
    autoCompleteMenu = this.$refs.autoCompleteMenu as typeof QMenu;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    inputField.value = this.$refs.autoCompleteInput as any;
  },
  setup: function () {
    return { inputText, inputField, autocompleteOptions, menuShowing };
  },
});
</script>
