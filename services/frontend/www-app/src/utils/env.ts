import GeolocationPolyfill from './GeolocationPolyfill';
import DeviceOrientationPolyfill from './DeviceOrientationPolyfill';
// One singleton to rule them all
export default {
  geolocation: new GeolocationPolyfill(),
  deviceOrientation: new DeviceOrientationPolyfill(),
};
