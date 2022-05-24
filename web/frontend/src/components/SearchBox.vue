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
      >
        <q-item-section>
          <q-item-label>{{ item.name }}</q-item-label>
          <q-item-label v-if="item.caption" caption>{{
            item.caption
          }}</q-item-label>
        </q-item-section>
      </q-item>
    </q-menu>
  </div>
</template>

<script lang="ts">
import { defineComponent, Ref, ref } from 'vue';
import { LongLat, POI } from 'components/models';
import { Event } from 'maplibre-gl';

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
    // FIXME: i18n? other places surely construct addresses differently
    var address = undefined;
    if (feature.properties.housenumber && feature.properties.street) {
      address = `${feature.properties.housenumber} ${feature.properties.street}`;
    } else if (feature.properties.street) {
      address = `${feature.properties.street}`;
    } else if (feature.properties.locality && feature.properties.city) {
      address = `${feature.properties.locality}, ${feature.properties.city}`;
    } else if (feature.properties.city) {
      address = `${feature.properties.city}`;
    }

    var locality = undefined;
    if (feature.properties.locality && feature.properties.city) {
      locality = `${feature.properties.locality}, ${feature.properties.city}`;
    } else if (feature.properties.city) {
      locality = `${feature.properties.city}`;
    }

    var name = feature.properties.name;
    var caption = undefined;
    if (name && address) {
      caption = address;
    } else if (name) {
      caption = undefined;
    } else if (address && locality) {
      name = address;
      caption = locality;
    } else if (address) {
      name = address;
    } else {
      continue;
    }
    const coordinates = feature?.geometry?.coordinates;
    const position: LongLat | undefined = coordinates
      ? { long: coordinates[0], lat: coordinates[1] }
      : undefined;
    options.push({
      name: name,
      caption: caption,
      key: feature.properties.osm_id,
      position: position,
    });
  }
  autocompleteOptions.value = options;
}

const isAndroid = /(android)/i.test(navigator.userAgent);

const autocompleteOptions: Ref<POI[]> = ref([]);

var poi: POI | null | undefined = null;

export default defineComponent({
  name: 'BaseMap',
  methods: {
    updateAutocompleteEventBeforeInput(event: Event) {
      const inputEvent = event as InputEvent;
      this.$refs.autoCompleteMenu.show();
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
    async updateAutocompleteEventRawString() {
      this.$refs.autoCompleteMenu.show();
      if (null !== poi) {
        poi = null;
        this.$emit('poi_selected', poi);
      }
      if (!isAndroid) {
        setTimeout(() => updateAutocomplete());
      }
    },
    clearAutocomplete() {
      this.$refs.autoCompleteMenu.hide();
      if (null !== poi) {
        poi = null;
        inputText.value = '';
        this.$emit('poi_selected', poi);
      }
    },
    updatePoi(item?: POI) {
      if (item !== poi) {
        poi = item;
        if (poi) {
          inputText.value = poi.name;
        } else {
          inputText.value = '';
        }
        this.$refs.autoCompleteMenu.hide();
        this.$emit('poi_selected', poi);
      }
    },
  },
  emits: ['poi_selected'],
  mounted: function () {
    inputField.value = this.$refs.autoCompleteInput;
  },
  setup: function () {
    return { inputText, inputField, autocompleteOptions, menuShowing };
  },
});
</script>
