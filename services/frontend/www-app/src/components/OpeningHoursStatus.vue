<template>
  <span v-if="openingHours.isOpen">
    <span v-if="openingHours.nextChange">
      {{
        openingHours.nextChangeIsToday
          ? $t('opening_hours_is_open_until_$time', {
              time: formattedTime,
            })
          : openingHours.nextChangeIsTomorrow
            ? $t('opening_hours_is_open_until_tomorrow_$time', {
                time: formattedTime,
              })
            : $t('opening_hours_is_open_until_later_$day_$time', {
                time: formattedTime,
                day: dayOfWeek(openingHours.nextChange),
              })
      }}
    </span>
    <span v-else>
      {{ $t('opening_hours_is_open') }}
    </span>
  </span>

  <span v-else>
    <span v-if="openingHours.nextChange">
      {{
        openingHours.nextChangeIsToday
          ? $t('opening_hours_is_closed_until_$time', {
              time: formattedTime,
            })
          : openingHours.nextChangeIsTomorrow
            ? $t('opening_hours_is_closed_until_tomorrow_$time', {
                time: formattedTime,
              })
            : $t('opening_hours_is_closed_until_later_$day_$time', {
                time: formattedTime,
                day: dayOfWeek(openingHours.nextChange),
              })
      }}
    </span>
    <span v-else>
      {{ $t('opening_hours_is_closed') }}
    </span>
  </span>
</template>

<script lang="ts">
import OpeningHours from 'src/models/OpeningHours';
import { defineComponent, PropType } from 'vue';
import { formatTimeTruncatingEmptyMinutes, dayOfWeek } from 'src/utils/format';

export default defineComponent({
  name: 'OpeningHoursStatus',
  props: {
    openingHours: {
      type: Object as PropType<OpeningHours>,
      required: true,
    },
  },
  computed: {
    formattedTime(): string | undefined {
      if (this.openingHours.nextChange) {
        return formatTimeTruncatingEmptyMinutes(this.openingHours.nextChange);
      } else {
        return undefined;
      }
    },
  },
  methods: {
    dayOfWeek,
  },
});
</script>
