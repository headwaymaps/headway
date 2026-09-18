import { Marker } from 'maplibre-gl';

export default {
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
  tripEnd: (): Marker => {
    return new Marker({ color: '#111111' });
  },
  /// A transit vehicle's live position: the emoji for its kind, ringed in the route's color and
  /// pulsing to say the position is live.
  ///
  /// `ageText` is called on hover rather than baked in, so the "as of" age is current at the
  /// moment the traveler reads it.
  transitVehicle: (options: {
    color: string;
    emoji: string;
    routeName: string;
    badge?: string;
    vehicleLabel?: string;
    /// Degrees clockwise from north. Omitted when the route's shape couldn't say.
    bearing?: number;
    ageText: () => string;
  }): Marker => {
    const element = document.createElement('div');
    element.className = 'transit-vehicle';

    const pulse = document.createElement('div');
    pulse.className = 'transit-vehicle__pulse';
    pulse.style.backgroundColor = options.color;

    // A rotated emoji just reads as broken, so the direction goes on a pointer that travels
    // around the chip's rim instead. The pointer starts at 12 o'clock, which is north.
    if (options.bearing !== undefined) {
      const heading = document.createElement('div');
      heading.className = 'transit-vehicle__heading';
      heading.style.transform = `rotate(${options.bearing}deg)`;
      const arrow = document.createElement('div');
      arrow.className = 'transit-vehicle__arrow';
      arrow.style.borderBottomColor = options.color;
      heading.append(arrow);
      element.append(heading);
    }

    const vehicle = document.createElement('div');
    vehicle.className = 'transit-vehicle__vehicle';
    vehicle.style.borderColor = options.color;
    vehicle.textContent = options.emoji;

    const route = document.createElement('div');
    route.className = 'transit-vehicle__route';
    const routeEmoji = document.createElement('span');
    routeEmoji.textContent = options.emoji;
    const routeName = document.createElement('span');
    routeName.textContent = options.routeName;
    route.append(routeEmoji, routeName);
    if (options.vehicleLabel) {
      const label = document.createElement('span');
      label.className = 'transit-vehicle__label';
      label.textContent = options.vehicleLabel;
      route.append(label);
    }

    const age = document.createElement('div');
    age.className = 'transit-vehicle__age';
    const realTime = document.createElement('i');
    realTime.className = 'material-icons transit-vehicle__realtime';
    realTime.textContent = 'rss_feed';
    const ageText = document.createElement('span');
    age.append(realTime, ageText);

    const tooltip = document.createElement('div');
    tooltip.className = 'transit-vehicle__tooltip';
    tooltip.append(route, age);

    element.append(pulse, vehicle);
    if (options.badge) {
      const badge = document.createElement('div');
      badge.className = 'transit-vehicle__badge';
      badge.style.borderColor = options.color;
      badge.textContent = options.badge;
      element.append(badge);
    }
    element.append(tooltip);
    element.addEventListener('mouseenter', () => {
      ageText.textContent = options.ageText();
    });

    return new Marker({ element });
  },
  /// Dim a vehicle marker, for one running a route the traveler hasn't selected.
  setTransitVehicleFaded: (marker: Marker, faded: boolean): void => {
    marker.getElement().classList.toggle('transit-vehicle--faded', faded);
  },

  /// Re-aim a vehicle marker without rebuilding it, as its bearing changes between polls.
  setTransitVehicleBearing: (marker: Marker, bearing?: number): void => {
    const heading = marker
      .getElement()
      .querySelector<HTMLElement>('.transit-vehicle__heading');
    if (heading && bearing !== undefined) {
      heading.style.transform = `rotate(${bearing}deg)`;
    }
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
