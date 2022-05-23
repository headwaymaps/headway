<template>
  <q-layout view="lHh Lpr lFf">
    <q-header elevated>
      <q-toolbar>
        <q-btn
          flat
          dense
          round
          icon="menu"
          aria-label="Menu"
          @click="toggleLeftDrawer"
        />

        <q-input
          class="mainSearchBar"
          label="Search"
          label-color="white"
          :input-style="{ color: 'white' }"
          :outlined="true"
          :model-value="searchBarText"
          v-on:update:model-value="updateAutocomplete"
        >
          <q-menu
            :no-focus="true"
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

        <div>Quasar v{{ $q.version }}</div>
      </q-toolbar>
    </q-header>

    <q-drawer
      v-model="leftDrawerOpen"
      bordered
    >
      <q-list>
        <q-item-label
          header
        >
          Essential Links
        </q-item-label>
      </q-list>
    </q-drawer>

    <q-page-container>
      <router-view />
    </q-page-container>
  </q-layout>
</template>

<script lang="ts">
import { defineComponent, reactive, ref, Ref } from 'vue';
import { AutocompleteResult } from 'components/models'

const autocompleteOptions: Ref<AutocompleteResult[]> = ref([])

export default defineComponent({
  name: 'MainLayout',

  components: {
  },

  methods: {
    async updateAutocomplete(value: string) {
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
        autocompleteOptions.value = options
      }
    }
  },

  setup () {
    const leftDrawerOpen = ref(false)
    const searchBarText = ref("")

    return {
      leftDrawerOpen,
      searchBarText,
      autocompleteOptions,
      toggleLeftDrawer () {
        leftDrawerOpen.value = !leftDrawerOpen.value
      }
    }
  }
});
</script>
