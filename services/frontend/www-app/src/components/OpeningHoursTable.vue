<template>
  <table>
    <tr
      v-for="dayRange in openingHours.weeklyRanges()"
      :key="JSON.stringify(dayRange)"
    >
      <td>{{ dayRange.day }}</td>
      <td>
        <ul v-if="dayRange.intervals.length > 0" class="opening-hours">
          <li
            v-for="interval in dayRange.intervals"
            :key="JSON.stringify(interval)"
          >
            {{ formatTimeTruncatingEmptyMinutes(interval[0]) }}
            -
            {{ formatTimeTruncatingEmptyMinutes(interval[1]) }}
          </li>
        </ul>
        <ul v-else class="opening-hours">
          <li>
            {{ $t('opening_hours_is_closed') }}
          </li>
        </ul>
      </td>
    </tr>
  </table>
</template>

<script lang="ts">
import OpeningHours from 'src/models/OpeningHours';
import { defineComponent, PropType } from 'vue';
import { formatTimeTruncatingEmptyMinutes } from 'src/utils/format';

export default defineComponent({
  name: 'OpeningHoursTable',
  props: {
    openingHours: {
      type: Object as PropType<OpeningHours>,
      required: true,
    },
  },
  methods: {
    formatTimeTruncatingEmptyMinutes,
  },
});
</script>

<style lang="scss">
ul.opening-hours {
  margin-block-start: 2px;
  margin-block-end: 2px;
  list-style: none;
}

// bold "today"
tr:first-child {
  font-weight: bold;
}
</style>
