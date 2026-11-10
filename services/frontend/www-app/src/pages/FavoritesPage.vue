<template>
  <div class="top-card">
    <q-btn flat icon="arrow_back" to="/" label="Map" /><span class="text-h6"
      >Favorites</span
    >
  </div>
  <div class="bottom-card">
    <template v-if="favorite && place">
      <p v-if="loading" class="q-px-md" role="status">
        Updating business details…
      </p>
      <p v-if="refreshFailed" class="q-px-md" role="status">
        Current business details are unavailable. Your saved name and
        description are still available.
      </p>
      <place-card :place="place" />
    </template>
    <template v-else>
      <p class="q-pa-md">
        {{
          favorites.length
            ? 'Choose a favorite to see its details.'
            : 'No favorites yet. Open a place and choose Save favorite.'
        }}
      </p>
      <q-list>
        <q-item
          v-for="item in favorites"
          :key="item.id"
          clickable
          :to="`/favorites/${item.id}`"
        >
          <q-item-section avatar
            ><q-icon name="star" color="amber-9"
          /></q-item-section>
          <q-item-section
            ><q-item-label>{{ item.name }}</q-item-label
            ><q-item-label caption>{{
              item.description || item.address
            }}</q-item-label></q-item-section
          >
        </q-item>
      </q-list>
    </template>
    <q-btn v-if="favorite" flat to="/favorites" label="All favorites" />
    <p class="q-pa-md text-caption">
      Favorites are saved in this browser on this device.
    </p>
  </div>
</template>
<script setup lang="ts">
import { computed, shallowRef, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import PlaceCard from 'src/components/PlaceCard.vue';
import {
  favorites,
  favoritePlace,
  refreshFavoritePlace,
} from 'src/models/Favorites';
import { baseMapPromise } from 'src/components/BaseMap.vue';
const route = useRoute();
const favorite = computed(() =>
  favorites.value.find((item) => item.id === route.params.favoriteId),
);
const place = shallowRef<ReturnType<typeof favoritePlace>>();
const loading = ref(false);
const refreshFailed = ref(false);
watch(
  favorite,
  async (value, _previous, onCleanup) => {
    let cancelled = false;
    onCleanup(() => {
      cancelled = true;
    });
    place.value = undefined;
    loading.value = false;
    refreshFailed.value = false;
    if (!value) return;
    // Show the personal fields immediately, without presenting old business
    // metadata as if it were current while the request is pending or fails.
    const saved = favoritePlace(value);
    saved.address = undefined;
    saved.phone =
      saved.website =
      saved.openingHours =
      saved.countryCode =
      saved.area =
        undefined;
    place.value = saved;
    loading.value = true;
    try {
      const latest = await refreshFavoritePlace(value);
      if (!cancelled) place.value = latest;
    } catch {
      if (!cancelled) refreshFailed.value = true;
    } finally {
      if (!cancelled) loading.value = false;
    }
  },
  { immediate: true },
);
watch(
  place,
  async (value) => {
    if (!value) return;
    const map = await baseMapPromise;
    if (place.value === value) {
      map.removeAllMarkers();
      map.removeAllLayers();
      map.flyToPlace(value);
    }
  },
  { immediate: true },
);
</script>
