import {
  Map,
  GeolocateControl,
  GeolocateControlOptions,
  Evented,
  IControl,
  Marker,
  LngLat,
} from 'maplibre-gl';
import env from 'src/utils/env';
import { i18n } from 'src/i18n/lang';

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
  controlContainer: HTMLElement;
  currentRotation = 0;
  compassMarker: Marker;

  mostRecentPosition?: LngLat | undefined;
  mostRecentCompassHeading?: number;
  map?: Map | undefined;

  constructor(options: GeolocateControlOptions) {
    super();
    this.geolocateControl = new GeolocateControl(options);

    const compassEl = document.createElement('div');
    compassEl.className = 'headway-device-orientation-indicator';
    const svgEl = buildSvgElement(40);
    this.svgEl = svgEl;
    compassEl.append(svgEl);
    this.compassEl = compassEl;
    this.controlContainer = document.createElement('div');
    this.controlContainer.className = 'headway-location-control-container';

    // create click interceptor div inside controlContainer. When it's clicked pass the clicks through
    // to the geolocate control button
    const clickInterceptor = document.createElement('div');
    clickInterceptor.className = 'headway-location-control-click-interceptor';
    clickInterceptor.addEventListener('click', () => {
      if (this.geolocateControl._container.querySelector('button[disabled]')) {
        this.onLocationServiceIsDisabled();
      } else {
        this.geolocateControl._container.querySelector('button')?.click();
      }
    });
    this.controlContainer.append(clickInterceptor);

    this.compassMarker = new Marker({
      element: compassEl,
      pitchAlignment: 'map',
      rotationAlignment: 'map',
    });
  }

  /** {@inheritDoc IControl.onAdd} */
  onAdd(map: Map) {
    this.map = map;
    env.deviceOrientation.subscribe(this._updateMarkerRotation.bind(this));

    const geolocateControlEl = this.geolocateControl.onAdd(map);
    this.geolocateControl.on(
      'geolocate',
      this._updateMarkerPosition.bind(this),
    );

    // make sure the compass is hidden when the user stops tracking their location
    this.geolocateControl.on(
      'trackuserlocationend',
      this._updateMarker.bind(this),
    );

    geolocateControlEl.addEventListener('click', () => {
      env.deviceOrientation.startWatching();
    });

    this._setupDisabledObserver(geolocateControlEl);

    this.controlContainer.append(geolocateControlEl);
    return this.controlContainer;
  }

  /** {@inheritDoc IControl.onRemove} */
  onRemove() {
    this.map = undefined;
    this.geolocateControl.off('geolocate', this._updateMarkerPosition);
    this.geolocateControl.off('geolocate', this._updateMarkerPosition);
    return this.geolocateControl.onRemove();
  }

  _updateMarkerPosition(position?: GeolocationPosition | null): void {
    if (position) {
      this.mostRecentPosition = new LngLat(
        position.coords.longitude,
        position.coords.latitude,
      );
    } else {
      console.assert(false, 'position should not be null');
      this.mostRecentPosition = undefined;
    }
    this._updateMarker();
  }

  _updateMarkerRotation(compassHeading: number): void {
    this.mostRecentCompassHeading = compassHeading;
    this._updateMarker();
  }

  _setupDisabledObserver(geolocateControlEl: HTMLElement): void {
    const observer = new MutationObserver((mutations) => {
      const button = geolocateControlEl.querySelector('button');
      if (!button) {
        // button not inserted yet
        return;
      }

      mutations.forEach((mutation) => {
        if (
          mutation.type === 'attributes' &&
          mutation.attributeName === 'disabled'
        ) {
          const isDisabled = button.getAttribute('disabled') !== null;
          if (isDisabled) {
            this.onLocationServiceIsDisabled();
          }
        }
      });
    });

    // Specify what to observe (attribute changes in this case)
    const config = { attributes: true, childList: true, subtree: true };
    observer.observe(geolocateControlEl, config);
  }

  onLocationServiceIsDisabled() {
    // abort if banner already exists
    if (
      this.controlContainer.querySelector('.headway-location-disabled-banner')
    ) {
      return;
    }
    const banner = document.createElement('p');
    banner.className = 'headway-location-disabled-banner';
    banner.textContent = i18n.global.t('location_permission_denied_banner');
    this.controlContainer.append(banner);

    this.map?.once('click', () => {
      banner.remove();
    });
    banner.addEventListener('click', () => {
      banner.remove();
    });
  }

  _updateMarker() {
    if (
      this.mostRecentPosition === undefined ||
      this.mostRecentCompassHeading == undefined ||
      this.map === undefined ||
      this.geolocateControl._watchState == 'OFF'
    ) {
      this.compassMarker.remove();
      return;
    }
    this.compassMarker.setLngLat(this.mostRecentPosition);
    if (this.compassMarker._map !== this.map) {
      this.compassMarker.addTo(this.map);
    }

    // Align our design asset to correspond with the compass offset
    const graphicOffset = 180.0 + 62;

    // Adjust the target angle for a smooth transition
    const targetAngle = (graphicOffset + this.mostRecentCompassHeading) % 360;

    // Avoid a janky animation when wrapping around 360->0
    const delta = LocationControl.nearestDelta(
      this.currentRotation,
      targetAngle,
    );
    const finalAngle = this.currentRotation + delta;
    // We use a CSS transform rather than Marker.rotation so that
    // we can smooth it out with an animation.
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

function buildSvgElement(coneLength: number): HTMLElement {
  const svgWidth = coneLength + 20;
  const circleRadius = svgWidth / 2;

  const svgText = `<svg width="${svgWidth}" height="${svgWidth}" xmlns="http://www.w3.org/2000/svg">
   <defs>
     <radialGradient id="strokeGradient" cx="50%" cy="50%" r="50%" fx="50%" fy="50%">
       <stop offset="50%" stop-color="#1da1f2" stop-opacity="0.7" /> <!-- inner -->
       <stop offset="100%" stop-color="#1da1f2" stop-opacity="0" /> <!-- outer-->
     </radialGradient>
   </defs>
   <circle r="${circleRadius}" cx="${circleRadius}" cy="${circleRadius}"
     fill="none"
     stroke="url(#strokeGradient)"
     stroke-width="${coneLength}"
     stroke-dasharray="30 ${svgWidth * 4}" />
   </svg>`;

  return new DOMParser().parseFromString(svgText, 'image/svg+xml')
    .documentElement;
}
