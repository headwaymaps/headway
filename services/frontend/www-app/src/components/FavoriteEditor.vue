<template>
  <q-card-section>
    <q-btn
      flat
      :icon="favorite ? 'star' : 'star_border'"
      :label="favorite ? 'Edit favorite' : 'Save favorite'"
      @click="open"
    />
    <p v-if="favorite?.description" class="favorite-description">
      {{ favorite.description }}
    </p>
    <q-dialog v-model="editing">
      <q-card style="width: 440px; max-width: 95vw">
        <q-form @submit="save">
          <q-card-section>
            <div class="text-h6">
              {{ favorite ? 'Edit favorite' : 'Save favorite' }}
            </div>
            <q-input
              v-model="name"
              label="Name"
              maxlength="200"
              :rules="[(value) => !!value.trim() || 'Enter a name']"
              autofocus
            />
            <q-input
              v-model="description"
              label="Description"
              type="textarea"
              maxlength="2000"
            />
            <p class="text-caption q-mt-sm">
              Saved in this browser on this device.
            </p>
            <p v-if="error" role="alert" class="text-negative">{{ error }}</p>
          </q-card-section>
          <q-card-actions align="right">
            <q-btn
              v-if="favorite"
              flat
              color="negative"
              label="Remove favorite"
              @click="remove"
            />
            <q-btn v-close-popup flat label="Cancel" />
            <q-btn type="submit" color="primary" label="Save" />
          </q-card-actions>
        </q-form>
      </q-card>
    </q-dialog>
  </q-card-section>
</template>
<script setup lang="ts">
import { computed, ref } from 'vue';
import Place from 'src/models/Place';
import {
  favoriteFor,
  saveFavorite,
  removeFavorite,
} from 'src/models/Favorites';
const props = defineProps<{ place: Place }>();
const favorite = computed(() => favoriteFor(props.place));
const editing = ref(false);
const name = ref('');
const description = ref('');
const error = ref('');
function open() {
  name.value =
    favorite.value?.name ??
    props.place.name ??
    props.place.address ??
    'Dropped pin';
  description.value = favorite.value?.description ?? '';
  error.value = '';
  editing.value = true;
}
function save() {
  try {
    saveFavorite(props.place, name.value, description.value);
    editing.value = false;
  } catch {
    error.value =
      'Could not save favorite. Check that browser storage is available.';
  }
}
function remove() {
  try {
    if (favorite.value) removeFavorite(favorite.value.id);
    editing.value = false;
  } catch {
    error.value =
      'Could not remove favorite. Check that browser storage is available.';
  }
}
</script>
<style scoped>
.favorite-description {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  margin: 8px 0 0;
}
</style>
