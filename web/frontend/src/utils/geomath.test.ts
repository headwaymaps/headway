import { suite } from 'uvu';
import { is } from 'uvu/assert';

import { fastDistanceMeters, fastPolylineDistanceMeters } from './geomath';

let result;

const fdm = suite('fastDistanceMeters');

fdm('Zero fast distance at 0, 0', () => {
  result = fastDistanceMeters({ long: 0, lat: 0 }, { long: 0, lat: 0 });

  is(result, 0);
});

fdm('Zero fast distance at 30, 30', () => {
  result = fastDistanceMeters({ long: 30, lat: 30 }, { long: 30, lat: 30 });

  is(result, 0);
});

fdm('E/W fast distance at 0, 0', () => {
  result = fastDistanceMeters({ long: 0, lat: 0 }, { long: 0.0001, lat: 0 });

  is(result.toFixed(2), (11.12).toFixed(2));
});

fdm('N/S fast distance at 0, 0', () => {
  result = fastDistanceMeters({ long: 0, lat: 0.0001 }, { long: 0, lat: 0 });

  is(result.toFixed(2), (11.12).toFixed(2));
});

fdm('Diagonal fast distance at 0, 0', () => {
  result = fastDistanceMeters(
    { long: 0, lat: 0.0001 },
    { long: -0.0001, lat: 0 }
  );

  is(result.toFixed(2), (15.73).toFixed(2));
});

fdm('E/W fast distance at 30, 30', () => {
  result = fastDistanceMeters(
    { long: 30, lat: 30 },
    { long: 30.0001, lat: 30 }
  );

  is(result.toFixed(2), (9.63).toFixed(2));
});

fdm('N/S fast distance at 30, 30', () => {
  result = fastDistanceMeters(
    { long: 30, lat: 30.0001 },
    { long: 30, lat: 30 }
  );

  is(result.toFixed(2), (11.12).toFixed(2));
});

fdm('Diagonal fast distance at 30, 30', () => {
  result = fastDistanceMeters(
    { long: 30, lat: 30.0001 },
    { long: 29.9999, lat: 30 }
  );

  is(result.toFixed(2), (14.71).toFixed(2));
});

fdm.run();

const fpdm = suite('fastPolylineDistanceMeters');

fpdm('Empty polyline distance zero', () => {
  result = fastPolylineDistanceMeters([]);

  is(result, 0);
});

fpdm('Single-point polyline distance zero', () => {
  result = fastPolylineDistanceMeters([{ long: 70, lat: 70 }]);

  is(result, 0);
});

fpdm('Colocated polyline distance zero', () => {
  result = fastPolylineDistanceMeters([
    { long: 70, lat: -70 },
    { long: 70, lat: -70 },
  ]);

  is(result, 0);
});

fpdm('Singe-segment polyline distance', () => {
  result = fastPolylineDistanceMeters([
    { long: 30, lat: 30 },
    { long: 30.0001, lat: 30 },
  ]);

  is(result.toFixed(2), (9.63).toFixed(2));
});

fpdm('Two-segment polyline distance', () => {
  result = fastPolylineDistanceMeters([
    { long: 30, lat: 30 },
    { long: 30.0001, lat: 30 },
    { long: 30.0002, lat: 30.0001 },
  ]);

  is(result.toFixed(2), (24.34).toFixed(2));
});

fpdm.run();
