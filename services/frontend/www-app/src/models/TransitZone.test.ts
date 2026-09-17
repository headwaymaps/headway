import { describe, expect, test } from 'vitest';
import { parseTransitZone } from './TransitZone';

function zone(overrides: Record<string, unknown> = {}): string {
  return JSON.stringify({
    version: 1,
    bounds: {
      min_lon: -123.265,
      min_lat: 46.933,
      max_lon: -121.601,
      max_lat: 48.999,
    },
    feeds: [
      { feed_onestop_id: 'f-c23-metrokingcounty', provider: 'Metro', url: '' },
      { feed_onestop_id: 'f-c23-soundtransit', provider: 'ST', url: '' },
    ],
    ...overrides,
  });
}

function messageFrom(text: string): string {
  const parsed = parseTransitZone(text);
  if (parsed.ok) {
    throw new Error('expected a parse failure');
  }
  return parsed.error.message;
}

describe('parseTransitZone', () => {
  test('reads the area and the selection back', () => {
    expect(parseTransitZone(zone())).toEqual({
      ok: true,
      value: {
        bbox: [-123.265, 46.933, -121.601, 48.999],
        feedIds: ['f-c23-metrokingcounty', 'f-c23-soundtransit'],
      },
    });
  });

  test('keeps feeds the index may no longer know, for the caller to report', () => {
    const parsed = parseTransitZone(
      zone({ feeds: [{ feed_onestop_id: 'f-c23-pugetsound~consolidated' }] }),
    );
    expect(parsed.ok && parsed.value.feedIds).toEqual([
      'f-c23-pugetsound~consolidated',
    ]);
  });

  test('refuses a schema version it would read past', () => {
    expect(messageFrom(zone({ version: 2 }))).toMatch(/version 2/);
  });

  test('refuses a file that is not a zone', () => {
    expect(messageFrom('not json')).toMatch(/valid JSON/);
    expect(messageFrom('"a string is not a zone"')).toMatch(/zone document/);
  });

  test('names the bounds field it is missing', () => {
    expect(
      messageFrom(zone({ bounds: { min_lon: -122, min_lat: 47 } })),
    ).toMatch(/max_lon/);
    expect(messageFrom(zone({ bounds: undefined }))).toMatch(/no bounds/);
  });

  test('refuses bounds it cannot draw', () => {
    expect(
      messageFrom(
        zone({
          bounds: { min_lon: -122, min_lat: 47, max_lon: -122, max_lat: 48 },
        }),
      ),
    ).toMatch(/empty bounds/);
  });

  test('says which feed has no id', () => {
    expect(
      messageFrom(
        zone({
          feeds: [{ feed_onestop_id: 'f-ok' }, { provider: 'nameless' }],
        }),
      ),
    ).toMatch(/Feed 2/);
    expect(messageFrom(zone({ feeds: undefined }))).toMatch(/no feeds/);
  });
});
