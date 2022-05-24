<template>
  <q-layout view="lHh Lpr lFf">
    <q-header elevated>
      <q-toolbar>
        <q-input
          class="mainSearchBar"
          label="Search"
          label-color="white"
          :input-style="{ color: 'white' }"
          :outlined="true"
          :debounce="100"
          :model-value="searchBarText"
          v-on:beforeinput="updateAutocompleteEventBeforeInput"
          v-on:update:model-value="updateAutocompleteEventRawString"
        >
          <q-menu
            persistent
            ref="autoCompleteMenu"
            :no-focus="true"
            :no-refocus="true"
          >
            <q-item :key="item.key" v-for="item in autocompleteOptions"
              clickable>
              <q-item-section>
                <q-item-label>{{ item.name }}</q-item-label>
                <q-item-label v-if="item.caption" caption>{{ item.caption }}</q-item-label>
              </q-item-section>
            </q-item>
          </q-menu>
        </q-input>
      </q-toolbar>
    </q-header>

    <q-page-container>
      <router-view />
    </q-page-container>
  </q-layout>
</template>

<script lang="ts">
import { defineComponent, reactive, ref, Ref } from 'vue';
import { AutocompleteResult } from 'components/models'

const isAndroid = /(android)/i.test(navigator.userAgent);

const autocompleteOptions: Ref<AutocompleteResult[]> = ref([])
const searchBarText = ref("")
var autoCompleteMenu = undefined;

async function updateAutocomplete(value_raw: string | number | null) {
  const value = `${value_raw}`
  searchBarText.value = value;
  const response = await fetch(`/photon/api?q=${encodeURIComponent(value)}&limit=10`)
  if (response.status != 200) {
    autocompleteOptions.value = []
    return
  }
  const results = await response.json()
  var options: AutocompleteResult[] = [];
  for (const feature of results.features) {
    // FIXME: i18n? other places surely construct addresses differently
    var address = undefined
    if (feature.properties.housenumber && feature.properties.street) {
      address = `${feature.properties.housenumber} ${feature.properties.street}`
    } else if (feature.properties.street) {
      address = `${feature.properties.street}`
    } else if (feature.properties.locality && feature.properties.city) {
      address = `${feature.properties.locality}, ${feature.properties.city}`
    } else if (feature.properties.city) {
      address = `${feature.properties.city}`
    }

    var locality = undefined
    if (feature.properties.locality && feature.properties.city) {
      locality = `${feature.properties.locality}, ${feature.properties.city}`
    } else if (feature.properties.city) {
      locality = `${feature.properties.city}`
    }

    var name = feature.properties.name
    var caption = undefined
    if (name && address) {
      caption = address
    } else if (name) {
      caption = undefined
    } else if (address && locality) {
      name = address
      caption = locality
    } else if (address) {
      name = address
    } else {
      continue
    }
    options.push({
      name: name,
      caption: caption,
      key: feature.properties.osm_id,
    })
  }
  autocompleteOptions.value = options
  console.log("showing")
  autoCompleteMenu.show();
}


export default defineComponent({
  name: 'MainLayout',
  
  mounted: function() {
    autoCompleteMenu = this.$refs.autoCompleteMenu;
  },

  components: {
  },

  methods: {
    async updateAutocompleteEventBeforeInput(event) {
      if (isAndroid) {
        await updateAutocomplete(event.target.value)
      }
    },
    async updateAutocompleteEventRawString(value_raw: string | number | null) {
      if (!isAndroid) {
        await updateAutocomplete(value_raw)
      }
    },
  },
  setup () {
    const leftDrawerOpen = ref(false)

    return {
      leftDrawerOpen,
      searchBarText,
      autocompleteOptions,
      toggleLeftDrawer () {
        leftDrawerOpen.value = !leftDrawerOpen.value
      },
    }
  }
});
</script>
