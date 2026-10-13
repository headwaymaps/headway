import { onMounted, onUnmounted, ref, watch } from 'vue';
import type { TripLeg } from 'src/models/Trip';
import {
  LegElevation,
  SteepSection,
  TravelmuxClient,
} from 'src/services/TravelmuxClient';
import { midpointFraction } from 'src/utils/grade';
import {
  focusSteepClimb,
  hideElevationScrubber,
  showElevationScrubber,
} from 'src/utils/Markers';
import { TravelMode } from 'src/utils/models';

/// A walking or cycling leg's elevation, and a scrubber shared between its chart and the map.
export function useLegElevation(leg: TripLeg) {
  const elevation = ref<LegElevation>();
  /// How far along the leg the scrubber is, as a fraction, or null when it's hidden.
  const scrubFraction = ref<number | null>(null);
  /// Set when the map already shows the scrubber where it's going: dragged on the map, the dot is
  /// under the pointer, and a picked climb zooms to fit.
  let mapIsPlaced = false;

  watch(scrubFraction, (fraction) => {
    if (fraction === null) {
      hideElevationScrubber();
      return;
    }
    const keepInView = !mapIsPlaced;
    mapIsPlaced = false;
    showElevationScrubber(
      leg,
      fraction,
      (dragged) => {
        mapIsPlaced = true;
        scrubFraction.value = dragged;
      },
      keepInView,
    );
  });

  onMounted(async () => {
    if (leg.mode !== TravelMode.Walk && leg.mode !== TravelMode.Bike) {
      return;
    }
    const result = await TravelmuxClient.legElevation(leg.raw);
    if (result.ok) {
      elevation.value = result.value;
    } else {
      console.error('Failed to fetch elevation data:', result.error);
    }
  });

  onUnmounted(hideElevationScrubber);

  /// Scrubs to the middle of `section` and shows it on the map.
  function selectSteepClimb(section: SteepSection) {
    mapIsPlaced = true;
    scrubFraction.value = midpointFraction(elevation.value!.profile, section);
    focusSteepClimb(section);
  }

  return { elevation, scrubFraction, selectSteepClimb };
}
