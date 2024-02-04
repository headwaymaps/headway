import {
  Map,
  GeolocateControl,
  GeolocateControlOptions,
  Evented,
  IControl,
} from 'maplibre-gl';
import env from 'src/utils/env';

/**
 * A wrapper for maplibre-gl's GeolocateControl that adds a compass to the user's location dot,
 * showing which way the user is facing. The compass is only supported on some
 * platforms - notably desktop devices likely do not have a compass, but the location dot will
 * still be shown.
 */
export default class LocationControl extends Evented implements IControl {
  geolocateControl: GeolocateControl;
  compassEl: HTMLElement;
  svgEl: HTMLElement;
  isAddedToDOM = false;
  currentRotation = 0;

  constructor(options: GeolocateControlOptions) {
    super();
    this.geolocateControl = new GeolocateControl(options);

    const compassEl = document.createElement('div');
    compassEl.className = 'headway-device-orientation-indicator';

    const svg = `<svg width="80" height="80" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <radialGradient id="strokeGradient" cx="50%" cy="50%" r="50%" fx="50%" fy="50%">
        <stop offset="50%" stop-color="#1da1f2" stop-opacity="0.7" /> <!-- inner -->
        <stop offset="100%" stop-color="#1da1f2" stop-opacity="0" /> <!-- outer-->
      </radialGradient>
    </defs>
    <circle r="40" cx="40" cy="40"
      fill="none"
      stroke="url(#strokeGradient)"
      stroke-width="70"
      stroke-dasharray="30 300" />
    </svg>`;
    const svgEl = new DOMParser().parseFromString(
      svg,
      'image/svg+xml',
    ).documentElement;

    this.svgEl = svgEl;
    compassEl.append(svgEl);
    this.compassEl = compassEl;
  }

  /** {@inheritDoc IControl.onAdd} */
  onAdd(map: Map) {
    env.deviceOrientation.subscribe((compassHeading) => {
      this.orientationDidUpdate(compassHeading);
    });
    const geolocateControlEl = this.geolocateControl.onAdd(map);

    geolocateControlEl.addEventListener('click', () => {
      env.deviceOrientation.startWatching();
    });

    return geolocateControlEl;
  }

  /** {@inheritDoc IControl.onRemove} */
  onRemove() {
    return this.geolocateControl.onRemove();
  }

  orientationDidUpdate(compassHeading: number) {
    if (this.geolocateControl._dotElement === undefined) {
      return;
    }

    if (!this.isAddedToDOM) {
      this.geolocateControl._dotElement.appendChild(this.compassEl);
      this.isAddedToDOM = true;
    }

    // Align our design asset to correspond with the compass offset
    const graphicOffset = 180.0 + 69;

    // Adjust the target angle for a smooth transition
    const targetAngle = (graphicOffset + compassHeading) % 360;

    // Avoid a janky animation when wrapping around 360->0
    const delta = LocationControl.nearestDelta(
      this.currentRotation,
      targetAngle,
    );
    const finalAngle = this.currentRotation + delta;
    this.svgEl.style.transform = `rotate(${finalAngle}deg)`;
    this.currentRotation = finalAngle;
  }

  // calculate the shortest distance between two angles
  static nearestDelta(from: number, to: number): number {
    const distance = (to - from) % 360;
    return distance > 180
      ? distance - 360
      : distance < -180
        ? distance + 360
        : distance;
  }
}
