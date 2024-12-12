import { Map } from 'maplibre-gl';

import type { ControlPosition, IControl } from 'maplibre-gl';
import Prefs from 'src/utils/Prefs';
import { DistanceUnits } from 'src/utils/models';
import lodash from 'lodash';

type ScaleOptions = {
  maxWidth?: number;
  units?: DistanceUnits;
};

const defaultOptions: ScaleOptions = {
  maxWidth: 100,
  units: DistanceUnits.Kilometers,
};

export default class ScaleControl implements IControl {
  _map?: Map | undefined;
  containerEl: HTMLElement;
  rulerEl: HTMLElement;
  textEl: HTMLElement;
  options: ScaleOptions;

  constructor(options: ScaleOptions) {
    if (!options.units && Prefs.stored.mapScaleDistanceUnits) {
      options.units = Prefs.stored.mapScaleDistanceUnits;
    }

    this.options = Object.assign({}, defaultOptions, options);
    this.containerEl = document.createElement('div');
    this.containerEl.className =
      'headway-ctrl-scale maplibregl-ctrl mapboxgl-ctrl';
    this.rulerEl = document.createElement('div');
    this.rulerEl.className = 'headway-ctrl-scale-ruler';
    this.textEl = document.createElement('div');
    this.textEl.className = 'headway-ctrl-scale-text';
    this.containerEl.appendChild(this.rulerEl);
    this.containerEl.appendChild(this.textEl);
    this.containerEl.addEventListener('click', () => {
      this.toggleUnits();
    });
  }

  getDefaultPosition(): ControlPosition {
    return 'bottom-left';
  }

  _onMove() {
    if (!this._map) {
      console.assert(false, 'map was unset');
      return;
    }
    this.updateScale();
  }

  onAdd(map: Map): HTMLElement {
    this._map = map;

    map.getContainer().appendChild(this.containerEl);

    const onMove = lodash.bind(this._onMove, this);
    this._map.on('move', onMove);

    this._onMove();

    return this.containerEl;
  }

  onRemove() {
    this.containerEl.remove();
    this._map?.off('move', this._onMove);
    this._map = undefined;
  }

  setDistanceUnits(units: DistanceUnits) {
    this.options.units = units;
    if (!this._map) {
      return;
    }
    this.updateScale();
  }

  toggleUnits() {
    if (this.options.units == DistanceUnits.Miles) {
      this.options.units = DistanceUnits.Kilometers;
    } else {
      this.options.units = DistanceUnits.Miles;
    }
    Prefs.stored.setMapScaleDistanceUnits(this.options.units);

    this.updateScale();
  }

  updateScale() {
    const map = this._map;
    if (!map) {
      console.assert(
        false,
        'tried updating scale without adding control to map',
      );
      return;
    }

    // A horizontal scale is imagined to be present at center of the map
    // container with maximum length (Default) as 100px.
    // Using spherical law of cosines approximation, the real distance is
    // found between the two coordinates.
    const maxWidth = (this.options && this.options.maxWidth) || 100;

    // Due to our projection, map scale varies with respect to latitude - e.g.
    // an inch near the equator covers more distance than an inch near the
    // poles.  Zoomed to city scale, it doesn't make much difference, but
    // when zoomed out to country or more, it's a big deal.
    //
    // So there's no "single" accurate scale we can show, so we have to choose
    // where to put our error.
    //
    // When I try to measure somewhat acurately, I pan the area I want to measure
    // near to the ruler. So, I think the most reasonable thing is to make sure
    // that the ruler is to scale with the land directly beneath it.
    const rulerBounds = this.rulerEl.getClientRects()[0]!;
    const y = rulerBounds.y;
    const left = map.unproject([0, y]);
    const right = map.unproject([maxWidth, y]);
    const maxMeters = left.distanceTo(right);

    // The real distance corresponding to 100px scale length is rounded off to
    // near pretty number and the scale length for the same is found out.
    if (this.options && this.options.units === DistanceUnits.Miles) {
      const maxFeet = 3.2808 * maxMeters;
      if (maxFeet > 5280) {
        const maxMiles = maxFeet / 5280;
        this.setScale(
          maxWidth,
          maxMiles,
          map._getUIString('ScaleControl.Miles'),
        );
      } else {
        this.setScale(maxWidth, maxFeet, map._getUIString('ScaleControl.Feet'));
      }
    } else if (maxMeters >= 1000) {
      this.setScale(
        maxWidth,
        maxMeters / 1000,
        map._getUIString('ScaleControl.Kilometers'),
      );
    } else {
      this.setScale(
        maxWidth,
        maxMeters,
        map._getUIString('ScaleControl.Meters'),
      );
    }
  }
  // TODO: scale to where ruler is (bottom of map)
  setScale(maxWidth: number, maxDistance: number, units: string) {
    const distance = getRoundNum(maxDistance);
    const ratio = distance / maxDistance;
    this.rulerEl.style.width = `${maxWidth * ratio}px`;
    this.textEl.innerHTML = `${distance}&nbsp;${units}`;
  }
}

function getDecimalRoundNum(d: number) {
  const multiplier = Math.pow(10, Math.ceil(-Math.log(d) / Math.LN10));
  return Math.round(d * multiplier) / multiplier;
}

function getRoundNum(num: number) {
  const pow10 = Math.pow(10, `${Math.floor(num)}`.length - 1);
  let d = num / pow10;

  d =
    d >= 10
      ? 10
      : d >= 5
        ? 5
        : d >= 3
          ? 3
          : d >= 2
            ? 2
            : d >= 1
              ? 1
              : getDecimalRoundNum(d);

  return pow10 * d;
}
