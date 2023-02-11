import { expect, test } from '@jest/globals';
import { LngLat } from 'maplibre-gl';
import { PlaceId } from './Place';

test('PlaceId.gid', () => {
  expect(PlaceId.gid('openstreetmap:venue:way/206623301').serialized()).toBe(
    'openstreetmap:venue:way/206623301'
  );

  expect(PlaceId.gid('openstreetmap:venue:way/206623301').urlEncoded()).toBe(
    'openstreetmap%3Avenue%3Away%2F206623301'
  );

  expect(PlaceId.deserialize('openstreetmap:venue:way/206623301').gid).toEqual(
    'openstreetmap:venue:way/206623301'
  );

  expect(
    PlaceId.deserialize('openstreetmap:venue:way/206623301').location
  ).toEqual(undefined);

  expect(PlaceId.urlDecoded('openstreetmap%3Avenue%3Away%2F206623301')).toEqual(
    PlaceId.gid('openstreetmap:venue:way/206623301')
  );
});

test('PlaceId.location', () => {
  expect(PlaceId.location(new LngLat(12.3, 45.6)).serialized()).toBe(
    '12.3,45.6'
  );

  expect(PlaceId.location(new LngLat(12.3, 45.6)).urlEncoded()).toBe(
    '12.3%2C45.6'
  );

  expect(PlaceId.deserialize('12.3,45.6').gid).toEqual(undefined);

  expect(PlaceId.deserialize('12.3,45.6').location).toEqual(
    new LngLat(12.3, 45.6)
  );

  expect(PlaceId.urlDecoded('12.3%2C45.6')).toEqual(
    PlaceId.location(new LngLat(12.3, 45.6))
  );
});
