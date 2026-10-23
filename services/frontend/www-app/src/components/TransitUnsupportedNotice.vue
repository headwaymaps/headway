<template>
  <div class="transit-unsupported">
    <q-icon name="directions_transit" size="40px" class="icon" />
    <div class="hw-heading">{{ $t('transit_unsupported_header') }}</div>
    <p v-if="transitAreaNames.length" class="hw-secondary">
      {{
        $t('transit_unsupported_available_in_$areas', {
          areas: formattedTransitAreas,
        })
      }}
    </p>
    <div v-if="notifyUrl" class="notify">
      <p>
        {{
          area
            ? $t('transit_unsupported_notify_$area', { area })
            : $t('transit_unsupported_notify')
        }}
      </p>
      <q-btn
        :href="notifyUrl"
        icon="mail"
        :label="$t('transit_unsupported_notify_btn')"
        color="primary"
        unelevated
        no-caps
        rounded
      />
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import Place from 'src/models/Place';
import Config from 'src/utils/Config';
import { TripEnd } from 'src/services/TravelmuxClient';

export default defineComponent({
  name: 'TransitUnsupportedNotice',
  props: {
    toPlace: { type: Object as PropType<Place>, default: undefined },
    fromPlace: { type: Object as PropType<Place>, default: undefined },
    transitAreaNames: { type: Array as PropType<string[]>, required: true },
    uncoveredPlaces: { type: Array as PropType<TripEnd[]>, required: true },
  },
  computed: {
    formattedTransitAreas(): string {
      return new Intl.ListFormat(this.$i18n.locale, {
        type: 'conjunction',
      }).format(this.transitAreaNames);
    },
    /// The area people would want transit added for: an end of the trip that has none.
    area(): string | undefined {
      const places = { from: this.fromPlace, to: this.toPlace };
      return this.uncoveredPlaces
        .map((end) => places[end]?.area)
        .find((area) => area);
    },
    /// Contact link with a subject naming the area, when the contact link is an email address.
    notifyUrl(): string | undefined {
      const contactUrl = Config.contactUrl;
      if (!contactUrl?.startsWith('mailto:')) {
        return contactUrl;
      }
      const address = contactUrl.slice('mailto:'.length).split('?')[0];
      const subject = this.area
        ? this.$t('transit_unsupported_email_subject_$area', {
            area: this.area,
          })
        : this.$t('transit_unsupported_email_subject');
      return `mailto:${address}?subject=${encodeURIComponent(subject)}`;
    },
  },
});
</script>

<style lang="scss">
.transit-unsupported {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 8px;
  padding: 24px 16px;

  .icon {
    opacity: 0.4;
  }

  .notify {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid rgba(0, 0, 0, 0.12);
    width: 100%;
  }
}
</style>
