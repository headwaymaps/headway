import { Platform } from 'quasar';
import type { GeolocatePositionEvent } from 'maplibre-gl';
import LocationControl from 'src/ui/LocationControl';

export default class GeolocationPolyfill {
  mostRecentPosition: GeolocationPosition | undefined;
  isWatching = false;
  isRegistered = false;

  // From navigator.geolocation
  //  clearWatch(watchId: number): void;
  //  getCurrentPosition(successCallback: PositionCallback, errorCallback?: PositionErrorCallback | null, options?: PositionOptions): void;
  //  watchPosition(successCallback: PositionCallback, errorCallback?: PositionErrorCallback | null, options?: PositionOptions): number;

  getCurrentPosition(
    successCallback: PositionCallback,
    errorCallback?: PositionErrorCallback | null,
    options?: PositionOptions,
  ): void {
    console.assert(
      this.isRegistered,
      'GeolocationPolyfill must be registered before calling getCurrentPosition',
    );

    // On macos Safari Version 16.2 (18614.3.7.1.5), after calling watchPosition, getCurrentPosition hangs indefinitely.
    // So we have this work-around
    //
    // To reproduce, delete the work around and:
    // 1. click bottom right to track user location
    // 2. type some location as your destination
    // 3. click directions
    // 4. for your "starting point" click "current location" button
    // 5.e Expect the "from" field to be filled in with "Current Location"
    // 5.a But actually, geolocation times out and the "from" field remains blank.
    //
    // Filling the "from" field via geolocation works so long as you aren't already
    // "watching"
    if (Platform.is.safari && Platform.is.mac && this.isWatching) {
      if (this.mostRecentPosition) {
        successCallback(this.mostRecentPosition);
      } else {
        // TODO: Tie into geolocationControl.on('geolocate') and wait for timeout?
        if (errorCallback) {
          console.error(
            'synthesizing GeolocationPositionError since no mostRecentLocation exists for polyfill',
          );
          const error = new GeolocationPositionError();
          errorCallback(error);
        }
      }
    } else {
      return navigator.geolocation.getCurrentPosition(
        successCallback,
        errorCallback,
        options,
      );
    }
  }

  register(geolocationControl: LocationControl): void {
    console.assert(
      !this.isRegistered,
      'GeolocationPolyfill has already been registered.',
    );

    geolocationControl.geolocateControl.on('geolocate', (event) => {
      const position = geolocationPosition(event);
      console.debug('updating mostRecentPosition', position);
      this.mostRecentPosition = position;
    });
    geolocationControl.geolocateControl.on(
      'trackuserlocationstart',
      (event) => {
        console.debug('starting user location watch', event);
        this.isWatching = true;
      },
    );
    geolocationControl.geolocateControl.on('trackuserlocationend', (event) => {
      console.debug('ending user location watch', event);
      switch (event.target._watchState) {
        case 'BACKGROUND': {
          // user panned, so screen is no longer locked to the user location, but location is still being upated.
          break;
        }
        case 'OFF': {
          this.isWatching = false;
          break;
        }
        default: {
          console.assert(
            false,
            `unexpected watch state: ${event.target._watchState}`,
            event.target,
          );
        }
      }
    });

    this.isRegistered = true;
  }
}

function geolocationPosition(
  event: GeolocatePositionEvent,
): GeolocationPosition {
  return {
    coords: event.coords,
    timestamp: event.timestamp,
    toJSON: () => ({
      coords: event.coords,
      timestamp: event.timestamp,
    }),
  };
}
