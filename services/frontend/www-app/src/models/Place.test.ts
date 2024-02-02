import { expect, test } from '@jest/globals';
import { LngLat } from 'maplibre-gl';
import { PlaceId } from './Place';
import OSMID from './OSMID';

test('PlaceId.gid', () => {
  expect(PlaceId.gid('openstreetmap:venue:way/206623301').serialized()).toBe(
    'openstreetmap:venue:way/206623301',
  );

  expect(PlaceId.gid('openstreetmap:venue:way/206623301').urlEncoded()).toBe(
    'openstreetmap%3Avenue%3Away%2F206623301',
  );

  expect(PlaceId.deserialize('openstreetmap:venue:way/206623301').gid).toEqual(
    'openstreetmap:venue:way/206623301',
  );

  expect(
    PlaceId.deserialize('openstreetmap:venue:way/206623301').location,
  ).toEqual(undefined);

  expect(PlaceId.urlDecoded('openstreetmap%3Avenue%3Away%2F206623301')).toEqual(
    PlaceId.gid('openstreetmap:venue:way/206623301'),
  );
});

test('PlaceId.location', () => {
  expect(PlaceId.location(new LngLat(12.3, 45.6)).serialized()).toBe(
    '12.3,45.6',
  );

  expect(PlaceId.location(new LngLat(12.3, 45.6)).urlEncoded()).toBe(
    '12.3%2C45.6',
  );

  expect(PlaceId.deserialize('12.3,45.6').gid).toEqual(undefined);

  expect(PlaceId.deserialize('12.3,45.6').location).toEqual(
    new LngLat(12.3, 45.6),
  );

  expect(PlaceId.urlDecoded('12.3%2C45.6')).toEqual(
    PlaceId.location(new LngLat(12.3, 45.6)),
  );
});

describe('PlaceId#osmVenueId', () => {
  test('happy path', () => {
    expect(
      PlaceId.deserialize('openstreetmap:venue:way/206623301').osmVenueId(),
    ).toEqual(OSMID.way(206623301));
  });

  test('typo in "way"', () => {
    expect(() =>
      PlaceId.deserialize('openstreetmap:venue:wa/206623301').osmVenueId(),
    ).toThrowError();
  });

  test('invalid numeric id', () => {
    expect(() =>
      PlaceId.deserialize('openstreetmap:venue:way/foobar').osmVenueId(),
    ).toThrowError();

    expect(() =>
      PlaceId.deserialize('openstreetmap:venue:way/0').osmVenueId(),
    ).toThrowError();

    expect(() =>
      PlaceId.deserialize('openstreetmap:venue:way/').osmVenueId(),
    ).toThrowError();
  });

  test('not a venue', () => {
    expect(
      PlaceId.deserialize('openstreetmap:foobar:way/206623301').osmVenueId(),
    ).toBeFalsy();
  });

  test('not OSM', () => {
    expect(
      PlaceId.deserialize('something:else:way/206623301').osmVenueId(),
    ).toBeFalsy();
  });
});

describe('PlaceId#editOSMVenueUrl', () => {
  test('node id', () => {
    expect(
      PlaceId.deserialize('openstreetmap:venue:node/1234')!
        .editOSMVenueUrl()!
        .toString(),
    ).toBe('https://www.openstreetmap.org/edit?editor=id&node=1234');
  });
  test('way id', () => {
    expect(
      PlaceId.deserialize('openstreetmap:venue:way/1234')!
        .editOSMVenueUrl()!
        .toString(),
    ).toBe('https://www.openstreetmap.org/edit?editor=id&way=1234');
  });
  test('not osm', () => {
    expect(
      PlaceId.deserialize('foobar:venue:node/1234')!.editOSMVenueUrl(),
    ).toBeUndefined();
  });
  test('not a venue', () => {
    expect(
      PlaceId.deserialize('openstreetmap:foobar:node/1234')!.editOSMVenueUrl(),
    ).toBeUndefined();
  });
  test('typo', () => {
    expect(() =>
      PlaceId.deserialize(
        'openstreetmap:venue:nodeeeeeeoops/1234',
      )!.editOSMVenueUrl(),
    ).toThrowError();
  });
  test('location, not GID', () => {
    expect(
      PlaceId.location(new LngLat(12.3, 45.6))!.editOSMVenueUrl(),
    ).toBeUndefined();
  });
});
