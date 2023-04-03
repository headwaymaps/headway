<template>
  <q-item class="row items-center" style="padding: 0">
    <q-icon class="col-1" size="sm" :name="icon" />
    <q-item-label class="col-" style="padding-left: 8px; padding-right: 8px">
      <slot />
    </q-item-label>
    <q-btn
      class="col-1"
      icon="content_copy"
      unelevated
      :ripple="false"
      size="sm"
      @click="copyToClipboard(copyText)"
    />
  </q-item>
</template>

<script lang="ts">
import { defineComponent } from 'vue';

export default defineComponent({
  name: 'PlaceField',
  props: {
    icon: {
      type: String,
      required: true,
    },
    copyText: {
      type: String,
      required: true,
    },
  },
  methods: {
    copyToClipboard(text: string): void {
      navigator.clipboard.writeText(text);
    },
  },
});
</script>
