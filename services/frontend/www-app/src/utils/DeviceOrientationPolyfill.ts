import { throttle } from 'lodash';
import { Platform } from 'quasar';

// Type definitions for DeviceOrientationEvent, which is not
// currently defined in the typescript standard library.
type PermissionStatus = 'granted' | 'denied' | 'prompt';

type State =
  | 'init'
  | 'requestingPermission'
  | 'permissionDenied'
  | 'unsupported'
  | 'watching';

let alreadySubscribedToFakeDeviceOrientation = false;

export default class DeviceOrientationPolyfill {
  private mostRecentHeading: number | null = null;
  private state: State = 'init';
  private subscribers: ((orientation: number) => void)[] = [];

  /**
   * Currently only Safari supports DeviceOrientationEvent.requestPermission. Other platforms allow
   * access to this information without explicit user permission.
   *
   * On iOS, this request must occur in response to a button press or else it
   * will be automatically denied without exposing any UI to the user.
   */
  requestPermission(): Promise<PermissionStatus> {
    this.state = 'requestingPermission';
    if (
      // @ts-expect-error: DeviceOrientationEvent.requestPermission is currently only defined in webkit
      typeof DeviceOrientationEvent.requestPermission === 'function'
    ) {
      // console.log('Calling actual DeviceOrientationEvent.requestPermission()');
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      return (DeviceOrientationEvent as any).requestPermission();
    } else {
      // console.log('Assuming permissions, since DeviceOrientationEvent.requestPermission does not exist');
      return Promise.resolve('granted');
    }
  }

  startWatching(): void {
    this.requestPermission().then((status) => {
      console.assert(
        this.state == 'requestingPermission',
        `unexpected state: ${this.state}`,
      );
      if (status === 'granted') {
        this.state = 'watching';

        // it's useful to be able to test this on desktop
        const fakeOnDesktop = false;
        if (fakeOnDesktop && Platform.is.desktop) {
          if (alreadySubscribedToFakeDeviceOrientation) {
            return;
          }
          alreadySubscribedToFakeDeviceOrientation = true;

          // console.log('fake device orientation for testing on desktop');
          setInterval(() => {
            // start north and turn clockwise
            let alpha;
            if (this.mostRecentHeading === null) {
              alpha = 0;
            } else {
              alpha = (360 - this.mostRecentHeading - 45) % 360;
            }

            // console.log(`mostRecentHeading: ${this.mostRecentHeading}, alpha: ${alpha}`);
            this.onDeviceOrientation({
              alpha,
              absolute: true,
            } as DeviceOrientationEvent);
          }, 1000);
        } else {
          // On iOS, listening to 'deviceorientation' produces angles with the webkitCompassHeading.
          // On Android it only produces angles with the non-absolute alphas, which are worthless to us,
          // so we listen to 'deviceorientationabsolute' instead.
          if (Platform.is.ios) {
            window.addEventListener('deviceorientation', (e) =>
              this.onDeviceOrientation(e),
            );
          } else {
            window.addEventListener('deviceorientationabsolute', (e) =>
              this.onDeviceOrientation(e as DeviceOrientationEvent),
            );
          }
        }
      } else {
        console.assert(status === 'denied', `unexpected status: ${status}`);
        this.state = 'permissionDenied';
      }
    });
  }

  stopWatching(): void {
    console.assert(
      this.state == 'unsupported',
      `unexpected state: ${this.state}`,
    );
    window.removeEventListener('deviceorientation', this.onDeviceOrientation);
    window.removeEventListener(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      'deviceorientationabsolute' as any,
      this.onDeviceOrientation,
    );
  }

  /**
   * Get notified when the compass heading changes.
   * 0 is north, 90 is east, 180 is south, 270 is west.
   *
   * Callback is throttled and only called when the change is significant.
   *
   * @param callback - called when the compass heading changes significantly
   */
  subscribe(callback: (compassHeading: number) => void): void {
    this.subscribers.push(callback);
  }

  // Maybe throttle less aggressively, and instead ensure it's "significant" change.
  private onDeviceOrientation = throttle(
    (event: DeviceOrientationEvent): void => {
      console.assert(
        this.state == 'watching',
        `unexpected state: ${this.state}`,
      );
      const significantChange = 1.0;

      // On iOS: listening to `deviceorientation` triggers events with
      // `webkitCompassHeading` which is what we ultimately need.
      // On Android: `deviceorientation` triggers events with an `alpha`
      // relative to whatever direction the user was initially facing.  This is
      // not useful to us, so we listen to `deviceorientationabsolute` instead.
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore
      let newHeading: number | null = null;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      if (typeof (event as any).webkitCompassHeading === 'number') {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        newHeading = (event as any).webkitCompassHeading;
      } else if (typeof event.alpha === 'number') {
        if (event.absolute) {
          // convert alpha to compass direction
          newHeading = (360 - event.alpha) % 360;
        } else {
          // Not interested in relative angles.
          return;
        }
      }

      if (newHeading == null) {
        console.assert(false, 'DeviceOrientationEvent missing angle');
        return;
      }

      if (
        this.mostRecentHeading &&
        Math.abs(newHeading - this.mostRecentHeading) < significantChange
      ) {
        // console.log('ignoring insignificant change');
        return;
      }

      // console.log(`setting heading to: ${newHeading}`);
      console.assert(newHeading >= 0);
      this.mostRecentHeading = newHeading;
      for (const subscriber of this.subscribers) {
        subscriber(this.mostRecentHeading);
      }
    },
    100,
  );
}
