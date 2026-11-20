<template>
  <ul class="alert-list">
    <li v-for="(group, idx) in groups" :key="groupKey(group, idx)">
      <div
        class="alert-header"
        :class="{ 'alert-header-expandable': isExpandable(group) }"
        @click.stop="isExpandable(group) && toggleGroup(idx)"
      >
        ⚠️ {{ groupLabel(group) }}
        <span v-if="group.alerts.length > 1" class="alert-count">
          {{ group.alerts.length }}
        </span>
        <q-icon
          v-if="isExpandable(group)"
          :name="isExpanded(idx) ? 'expand_less' : 'expand_more'"
        />
      </div>
      <ul v-if="isExpandable(group) && isExpanded(idx)" class="alert-details">
        <li v-for="alert in group.alerts" :key="alert.descriptionText">
          {{ alert.descriptionText.trim() }}
          <a
            v-if="alert.url"
            :href="alert.url"
            target="_blank"
            rel="noopener noreferrer"
            @click.stop
            >{{ $t('transit_alert_more_info') }}</a
          >
        </li>
      </ul>
    </li>
  </ul>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { TransitAlertGroup } from 'src/models/Trip';

/// Transit alerts, one row per group, each opening to its details.
export default defineComponent({
  name: 'TransitAlertList',
  props: {
    groups: {
      type: Array as PropType<TransitAlertGroup[]>,
      required: true,
    },
  },
  data(): { expandedAlertGroups: number[] } {
    return { expandedAlertGroups: [] };
  },
  methods: {
    // A group is only worth expanding when its header stands in for something else - a lone
    // headerless alert already shows its full text as the label.
    isExpandable(group: TransitAlertGroup): boolean {
      return group.headerText !== undefined;
    },
    groupLabel(group: TransitAlertGroup): string {
      return (
        group.headerText ??
        group.alerts[0]?.descriptionText ??
        ''
      ).trim();
    },
    groupKey(group: TransitAlertGroup, idx: number): string {
      return `${idx}-${group.headerText ?? ''}`;
    },
    isExpanded(idx: number): boolean {
      return this.expandedAlertGroups.includes(idx);
    },
    toggleGroup(idx: number) {
      if (this.isExpanded(idx)) {
        this.expandedAlertGroups = this.expandedAlertGroups.filter(
          (expanded) => expanded !== idx,
        );
      } else {
        this.expandedAlertGroups = [...this.expandedAlertGroups, idx];
      }
    },
  },
});
</script>

<style lang="scss">
.alert-list {
  list-style: none;
  padding: 0;
}
.alert-header-expandable {
  cursor: pointer;
}
.alert-count {
  opacity: 0.7;
  font-size: 0.85em;
  border: 1px solid currentcolor;
  border-radius: 999px;
  padding: 0 5px;
}
.alert-details {
  list-style: none;
  padding: 0 0 0 1.5em;
  opacity: 0.9;
  font-size: 0.9em;

  li + li {
    margin-top: 6px;
  }
}
</style>
