import GeolocationPolyfill from './GeolocationPolyfill';

// One singleton to rule them all
export default {
  geolocation: new GeolocationPolyfill(),
};
