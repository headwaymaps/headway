import { LngLatBounds, Marker, Popup } from 'maplibre-gl';
import { getBaseMap } from 'src/components/BaseMap.vue';
import type { TripLeg } from 'src/models/Trip';
import type { SteepSection } from 'src/services/TravelmuxClient';
import { decodePolyline } from 'src/utils/decodePolyline';
import {
  annotatedSteepClimbs,
  describeGrade,
  gradeColor,
} from 'src/utils/grade';

const Markers = {
  active: (): Marker => {
    const marker = new Marker({ color: '#111111' });
    marker.getElement().classList.add('cursor-pointer');
    return marker;
  },
  inactive: (): Marker => {
    const marker = new Marker({ color: '#11111155' });
    marker.getElement().classList.add('cursor-pointer');
    return marker;
  },
  transfer: (): Marker => {
    const element = document.createElement('div');
    element.innerHTML =
      '<svg display="block" height="15" width="15"><circle cx="8" cy="8" r="5" stroke="#888" stroke-width="2" fill="white" /></svg>';
    return new Marker({ element });
  },
  tripStart: (): Marker => {
    const element = document.createElement('div');
    element.innerHTML =
      '<svg display="block" height="20" width="20"><circle cx="10" cy="10" r="7" stroke="#111" stroke-width="2" fill="white" /></svg>';
    return new Marker({ element });
  },
  elevationScrubber: (): Marker => {
    const element = document.createElement('div');
    element.innerHTML =
      '<svg display="block" height="16" width="16"><circle cx="8" cy="8" r="5" stroke="#111" stroke-width="2" fill="white" /></svg>';
    element.style.cursor = 'grab';
    return new Marker({ element, draggable: true });
  },
  /// A pill showing the grade, like "9% ↗", standing where a steep climb begins. Clicking it says
  /// how steep the climb is.
  steepClimb: (grade: number): Marker => {
    const element = document.createElement('div');
    element.className = 'steep-climb-badge';
    element.style.background = gradeColor(grade);
    element.innerHTML = `${Math.round(grade * 100)}<span class="steep-climb-badge-percent">%</span><span class="steep-climb-badge-arrow">↗</span>`;
    const popup = new Popup({
      anchor: 'bottom',
      offset: [0, -22],
      closeButton: false,
    }).setText(describeGrade(grade));
    return new Marker({ element, anchor: 'bottom', offset: [0, -2] }).setPopup(
      popup,
    );
  },
  tripEnd: (): Marker => {
    return new Marker({ color: '#111111' });
  },
  maneuver: (icon: string, rotation: number = 0): Marker => {
    const element = document.createElement('div');
    element.innerHTML = `
      <div style="
        background: #2196f3;
        border: 2px solid #1976d2;
        border-radius: 50%;
        width: 24px;
        height: 24px;
        display: flex;
        align-items: center;
        justify-content: center;
        transform: rotate(${rotation}deg);
      ">
        <i class="material-icons" style="font-size: 14px; color: white;">${icon}</i>
      </div>
    `;
    return new Marker({ element });
  },
};

const ELEVATION_SCRUBBER_KEY = 'elevation-scrubber';

/// The scrubber on the map, kept across updates so it can be dragged.
let elevationScrubber:
  | { marker: Marker; leg: TripLeg; onDrag: (fraction: number) => void }
  | undefined;

/// Marks the point `fraction` of the way along `leg`. Dragging the mark slides it along `leg`,
/// reporting where it is to `onDrag`.
export function showElevationScrubber(
  leg: TripLeg,
  fraction: number,
  onDrag: (fraction: number) => void,
) {
  const map = getBaseMap();
  if (!map) {
    return;
  }
  if (!elevationScrubber || !map.hasMarker(ELEVATION_SCRUBBER_KEY)) {
    const marker = Markers.elevationScrubber();
    marker.on('drag', () => {
      if (!elevationScrubber) {
        return;
      }
      const { leg, onDrag } = elevationScrubber;
      const dragged = leg.fractionNearest(marker.getLngLat());
      marker.setLngLat(leg.pointAlong(dragged));
      onDrag(dragged);
    });
    elevationScrubber = { marker, leg, onDrag };
    marker.setLngLat(leg.pointAlong(fraction));
    map.pushMarker(ELEVATION_SCRUBBER_KEY, marker);
    return;
  }
  elevationScrubber.leg = leg;
  elevationScrubber.onDrag = onDrag;
  elevationScrubber.marker.setLngLat(leg.pointAlong(fraction));
}

export function hideElevationScrubber() {
  getBaseMap()?.removeMarker(ELEVATION_SCRUBBER_KEY);
  elevationScrubber = undefined;
}

const steepClimbBadges = new Map<string, Marker>();

function steepClimbBadgeKey(section: SteepSection): string {
  return `steep-climb-${section.geometry}`;
}

/// Marks where each steep climb on `legs` begins, replacing any previous badges. Returns the
/// marker keys, for pages that prune markers they don't own.
export function showSteepClimbBadges(legs: Pick<TripLeg, 'raw'>[]): string[] {
  const map = getBaseMap();
  if (!map) {
    return [];
  }
  steepClimbBadges.forEach((_, key) => map.removeMarker(key));
  steepClimbBadges.clear();
  for (const leg of legs) {
    const sections = leg.raw.nonTransitLeg?.elevation?.steepSections ?? [];
    for (const section of annotatedSteepClimbs(sections)) {
      const start = decodePolyline(section.geometry, 6)[0];
      if (!start) {
        continue;
      }
      const key = steepClimbBadgeKey(section);
      const marker = Markers.steepClimb(section.averageGrade).setLngLat([
        start[0]!,
        start[1]!,
      ]);
      map.pushMarker(key, marker);
      steepClimbBadges.set(key, marker);
    }
  }
  return [...steepClimbBadges.keys()];
}

/// Zooms the map to `section`, highlighting its badge over the others.
export function focusSteepClimb(section: SteepSection) {
  const map = getBaseMap();
  const points = decodePolyline(section.geometry, 6);
  if (!map || points.length === 0) {
    return;
  }
  const focusedKey = steepClimbBadgeKey(section);
  steepClimbBadges.forEach((marker, key) => {
    const isFocused = key === focusedKey;
    const element = marker.getElement();
    element.style.zIndex = isFocused ? '1' : '';
    element.classList.toggle('steep-climb-badge--focused', isFocused);
  });
  const bounds = points.reduce(
    (bounds, point) => bounds.extend(point),
    new LngLatBounds(points[0], points[0]),
  );
  map.fitBounds(bounds, { padding: 80, maxZoom: 17 });
}

export default Markers;
