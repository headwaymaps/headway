import { LngLat, Marker } from 'maplibre-gl';
import { h, render, shallowReactive, watchEffect } from 'vue';
import type { BaseMapInterface } from 'src/components/BaseMap.vue';
import TransitVehicleMarker, {
  TransitVehicleMarkerProps,
} from 'src/components/TransitVehicleMarker.vue';
import Trip from 'src/models/Trip';
import TransitVehicle from 'src/models/TransitVehicle';
import { PatternRequest, TravelmuxClient } from 'src/services/TravelmuxClient';

/// OTP polls its GTFS-RT vehicle position updaters once a minute, so asking much more often than
/// this just re-fetches a position we already have.
const POLL_INTERVAL_MS = 30_000;

/// A vehicle we're drawing, and the marker drawing it. The marker outlives a poll so it can be
/// animated between them - and so a tooltip being read doesn't vanish underneath the reader.
///
/// `props` is what a refresh writes to: `stopRendering`'s effect re-renders the marker's
/// component from it, so nothing here reaches into the marker's DOM.
interface TrackedVehicle {
  marker: Marker;
  element: HTMLElement;
  props: TransitVehicleMarkerProps;
  stopRendering: () => void;
}

/// Polls for the vehicles serving the transit legs of the trips on screen, draws each as a
/// pulsing dot, and walks it along travelmux's predicted track between polls until [stop]ped.
///
/// Only some feeds publish positions, so most trips draw nothing at all.
export default class VehicleOverlay {
  private map: BaseMapInterface;
  private from: LngLat;
  private to: LngLat;
  private trips: Trip[];
  private timer?: ReturnType<typeof setInterval>;
  private animation?: number;
  private tracked: Map<string, TrackedVehicle> = new Map();
  /// Milliseconds between this device's clock and the one that timed the tracks. A device even a
  /// few minutes out would otherwise hold every vehicle at one end of its track.
  private clockOffsetMs = 0;
  /// The patterns of the trip the traveler has picked. Vehicles on any other trip's patterns are
  /// drawn dimmed. Empty means nothing is picked yet, and nothing is dimmed.
  private selected: Set<string> = new Set();
  /// Asked to select the trip a clicked vehicle runs. The page owns which trip is selected, so
  /// an overlay whose page has nothing to select - one drawn over a single trip - leaves it unset.
  private didClickTrip?: (trip: Trip) => void;

  constructor(
    map: BaseMapInterface,
    from: LngLat,
    to: LngLat,
    trips: Trip[],
    didClickTrip?: (trip: Trip) => void,
  ) {
    this.map = map;
    this.from = from;
    this.to = to;
    this.trips = trips;
    this.didClickTrip = didClickTrip;
  }

  /// Bring the picked trip's vehicles forward and dim the rest.
  selectTrip(trip?: Trip): void {
    this.selected = new Set(trip?.patternCodes ?? []);
    for (const tracked of this.tracked.values()) {
      tracked.props.faded = this.isFaded(tracked.props.vehicle.raw.patternCode);
    }
  }

  private isFaded(patternCode: string): boolean {
    return this.selected.size > 0 && !this.selected.has(patternCode);
  }

  /// The trip a click on a vehicle running this pattern should select: none when the rider is
  /// already on it, and none when no trip on screen runs the pattern at all.
  tripToSelect(patternCode: string): Trip | undefined {
    if (this.selected.has(patternCode)) {
      return undefined;
    }
    return this.trips.find((trip) => trip.patternCodes.includes(patternCode));
  }

  private clickVehicle(key: string, props: TransitVehicleMarkerProps): void {
    this.pin(key);
    const trip = this.tripToSelect(props.vehicle.raw.patternCode);
    if (trip) {
      this.didClickTrip?.(trip);
    }
  }

  /// Hold one vehicle's popover open, releasing whatever was held before.
  ///
  /// The markers themselves carry which one it is, so a vehicle that stops being drawn takes its
  /// pin with it.
  private pin(key?: string): void {
    for (const [candidate, tracked] of this.tracked) {
      tracked.props.pinned = candidate === key;
    }
  }

  /// A click the marker didn't claim landed on the map or on something else drawn over it.
  private dismissPin = (): void => this.pin();

  start(): void {
    this.stop();
    document.addEventListener('click', this.dismissPin);
    // Before the first refresh, so that an in-flight poll can tell it's still wanted.
    this.timer = setInterval(() => void this.poll(), POLL_INTERVAL_MS);
    void this.poll();
    this.animate();
  }

  stop(): void {
    document.removeEventListener('click', this.dismissPin);
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = undefined;
    }
    if (this.animation !== undefined) {
      cancelAnimationFrame(this.animation);
      this.animation = undefined;
    }
    this.clearMarkers();
  }

  /// The patterns on screen and where the rider boards each: travelmux reports the vehicles
  /// either side of that stop rather than every vehicle running the route.
  private patternsToPoll(): PatternRequest[] {
    const boardingStops = new Map<string, LngLat>();
    for (const trip of this.trips) {
      for (const leg of trip.legs) {
        const patternCode = leg.raw.transitLeg?.patternCode;
        if (patternCode) {
          boardingStops.set(patternCode, leg.sourceLngLat);
        }
      }
    }
    return [...boardingStops].map(([code, boardingStop]) => ({
      code,
      boardingStop,
    }));
  }

  private clearMarkers(): void {
    for (const key of [...this.tracked.keys()]) {
      this.removeMarker(key);
    }
  }

  private removeMarker(key: string): void {
    const tracked = this.tracked.get(key);
    if (tracked) {
      tracked.stopRendering();
      render(null, tracked.element);
      this.tracked.delete(key);
    }
    this.map.removeMarker(key);
  }

  /// Walk every dot along its track. Positions are interpolated per frame rather than per poll,
  /// which is the whole point: a vehicle reports about once a minute but moves continuously.
  private animate(): void {
    const frame = () => {
      const now = Date.now() + this.clockOffsetMs;
      for (const tracked of this.tracked.values()) {
        tracked.marker.setLngLat(tracked.props.vehicle.positionAt(now));
      }
      this.animation = requestAnimationFrame(frame);
    };
    this.animation = requestAnimationFrame(frame);
  }

  /// [refresh], with anything it throws logged rather than left as an unhandled rejection -
  /// nothing awaits it, since it's driven by a timer.
  private async poll(): Promise<void> {
    try {
      await this.refresh();
    } catch (e) {
      console.warn('vehicle position refresh failed', e);
    }
  }

  private async refresh(): Promise<void> {
    const result = await TravelmuxClient.fetchVehiclePositions(
      this.from,
      this.to,
      this.patternsToPoll(),
    );

    if (!result.ok) {
      console.warn('failed to fetch vehicle positions', result.error);
      return;
    }

    // A poll that lands after the page has moved on has nothing left to draw onto.
    if (!this.timer) {
      return;
    }

    this.clockOffsetMs = result.value.clockOffsetMs;

    const reported = new Set<string>();
    for (const raw of result.value.vehicles) {
      const vehicle = new TransitVehicle(raw);
      const key = vehicle.markerKey;
      reported.add(key);

      const existing = this.tracked.get(key);
      if (existing) {
        // Keep the marker: re-creating it restarts the pulse and drops any open tooltip.
        existing.props.vehicle = vehicle;
        existing.props.clockOffsetMs = this.clockOffsetMs;
        existing.props.faded = this.isFaded(raw.patternCode);
        continue;
      }

      // Shallow, because replacing the vehicle is the only change worth a re-render - and a deep
      // proxy would wrap the vehicle's track a point at a time.
      const props: TransitVehicleMarkerProps = shallowReactive({
        vehicle,
        clockOffsetMs: this.clockOffsetMs,
        faded: this.isFaded(raw.patternCode),
        pinned: false,
      });
      const element = document.createElement('div');
      // A component mounted by hand has no parent re-rendering it, so props alone would be read
      // once and never again. Spreading them inside an effect is that missing parent: every key
      // is a dependency, so writing one re-renders the marker.
      const stopRendering = watchEffect(() => {
        render(
          h(TransitVehicleMarker, {
            ...props,
            onPin: () => this.clickVehicle(key, props),
          }),
          element,
        );
      });
      const marker = new Marker({ element }).setLngLat(
        vehicle.positionAt(Date.now() + this.clockOffsetMs),
      );
      this.tracked.set(key, { marker, element, props, stopRendering });
      this.map.pushMarker(key, marker);
    }

    // A vehicle a poll didn't mention is usually a gap in the feed or a wobble in what travelmux
    // ranks as nearby, not a bus that went away - so it keeps coasting along the track it already
    // has, and is only dropped once that track is spent.
    const now = Date.now() + this.clockOffsetMs;
    for (const [key, tracked] of [...this.tracked]) {
      if (!reported.has(key) && tracked.props.vehicle.hasExpiredAt(now)) {
        this.removeMarker(key);
      }
    }
  }
}
