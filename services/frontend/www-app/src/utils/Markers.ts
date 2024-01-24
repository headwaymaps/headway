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
};
