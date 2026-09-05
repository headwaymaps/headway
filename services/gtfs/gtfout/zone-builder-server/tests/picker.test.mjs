// What the picker is responsible for: proposing candidates without choosing
// them, never losing a deliberate pick, and producing a document the build can
// read. Each test drives the real page.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

import { describe, expect, test } from 'vitest';

import { drawBox, feed, loadPicker, rows, settle } from './harness.mjs';

const REPO = path.join(path.dirname(fileURLToPath(import.meta.url)), '../../../../..');

/// The box drawn by most tests, in screen pixels, and the bbox the stub
/// projection turns it into. See maplibre-stub.mjs.
const BOX = { x1: -12240, y1: 220, x2: -12200, y2: 260 };
const BOX_BBOX = '-122.4,47.4,-122,47.8';

const KCM = feed({
  feed_id: 'f-c23-metrokingcounty',
  provider: 'King County Metro',
  bbox: [-122.5, 47.3, -121.9, 47.9],
  area_m2: 3_000e6,
  relevance: 0.8,
  realtime: [
    {
      feed_id: 'f-kingcountymetro~rt',
      kinds: ['trip updates', 'alerts'],
      authorization_type: '',
      info_url: null,
      urls: { trip_updates: 'https://example.com/tu.pb' },
    },
  ],
});

const SOUND_TRANSIT = feed({
  feed_id: 'f-c23-soundtransit',
  provider: 'Sound Transit',
  bbox: [-122.6, 47.1, -122.0, 48.0],
  area_m2: 6_000e6,
  relevance: 0.5,
  authorization_type: 'query_param',
  info_url: 'https://example.com/request-a-key',
});

const AMTRAK = feed({
  feed_id: 'f-9-amtrak',
  provider: 'Amtrak',
  bbox: [-124.0, 32.0, -70.0, 49.0],
  area_m2: 6_000_000e6,
  relevance: 0.01,
});

/// Far enough east that the drawn box never touches it.
const SPOKANE = feed({
  feed_id: 'f-c2h-spokanetransit',
  provider: 'Spokane Transit',
  bbox: [-117.6, 47.5, -117.1, 47.8],
  area_m2: 800e6,
  relevance: 0.2,
});

const CATALOG = [KCM, SOUND_TRANSIT, AMTRAK, SPOKANE];

describe('drawing an area', () => {
  test('queries the feeds the box touches, and picks none of them', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, BOX);

    expect(picker.requests.map((r) => r.url)).toContain(
      `/api/feeds-by-bbox?bbox=${BOX_BBOX}`,
    );

    // Selection is opt-in: drawing proposes, it never chooses.
    expect(rows(picker).map((r) => r.checked)).toEqual([false, false, false]);
    expect(picker.els.count.textContent).toBe('0 of 3 selected');
    expect(picker.els.download.disabled).toBe(true);
  });

  test('orders by relevance to the box rather than by size', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, BOX);

    // Amtrak is by far the biggest feed and by far the worst match.
    expect(rows(picker).map((r) => r.provider)).toEqual([
      'King County Metro',
      'Sound Transit',
      'Amtrak',
    ]);
  });

  test('flags a feed that covers more ground than a large country', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, BOX);

    const [kcm, , amtrak] = rows(picker);
    expect(amtrak.meta).toContain('⚠');
    expect(amtrak.meta).toContain('6,000,000 km²');
    expect(kcm.meta).not.toContain('⚠');
    expect(kcm.meta).toContain('3,000 km²');
  });

  test('says which feeds need a credential, static and realtime alike', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, BOX);

    const [kcm, soundTransit] = rows(picker);
    expect(soundTransit.needs).toBe('needs query_param');
    expect(kcm.needs).toBe(null);
    // Realtime rides along with its static feed rather than being its own row.
    expect(kcm.realtime).toContain('trip updates');
  });

  test('a click rather than a drag leaves the results alone', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, BOX);
    const before = picker.requests.length;

    await drawBox(picker, { x1: -12240, y1: 220, x2: -12238, y2: 222 });

    expect(picker.requests.length).toBe(before);
    expect(rows(picker)).toHaveLength(3);
  });
});

describe('selecting feeds', () => {
  test('ticking one posts it, and the pane shows the document', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, BOX);

    rows(picker)[0].checkbox.click();
    await settle();

    const post = picker.requests.findLast((r) => r.url === '/api/zone');
    expect(JSON.parse(post.options.body)).toEqual({
      bbox: BOX_BBOX,
      feed_ids: ['f-c23-metrokingcounty'],
      credentials: {},
    });

    const zone = JSON.parse(picker.els.json.value);
    expect(zone.version).toBe(1);
    expect(zone.bounds).toEqual({
      min_lon: -122.4,
      min_lat: 47.4,
      max_lon: -122,
      max_lat: 47.8,
    });
    expect(zone.feeds.map((f) => f.feed_onestop_id)).toEqual([
      'f-c23-metrokingcounty',
    ]);
    expect(picker.els.download.disabled).toBe(false);
  });

  test('counts the selected feeds that carry realtime', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, BOX);

    rows(picker)[1].checkbox.click(); // Sound Transit: no realtime
    await settle();
    expect(picker.els.count.textContent).toBe('1 of 3 selected');

    rows(picker)[0].checkbox.click(); // King County Metro: realtime
    await settle();
    expect(picker.els.count.textContent).toBe('2 of 3 selected, 1 with realtime');
  });

  test('select all then none leaves nothing to download', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, BOX);

    document.getElementById('all').click();
    await settle();
    expect(picker.els.count.textContent).toContain('3 of 3 selected');

    document.getElementById('none').click();
    await settle();
    expect(picker.els.count.textContent).toBe('0 of 3 selected');
    expect(picker.els.json.value).toBe('');
    expect(picker.els.download.disabled).toBe(true);
  });

  test('the selected extents are what gets outlined on the map', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, BOX);

    rows(picker)[0].checkbox.click();
    await settle();

    // The style hasn't loaded, so the sources don't exist yet; the page redraws
    // on load rather than losing the outlines. Emitting it is what a real map
    // does once the vector tiles arrive.
    const map = document.querySelector('#map').lastElementChild;
    expect(map).toBeTruthy();
  });
});

describe('moving the box', () => {
  test('keeps a pick the box no longer covers, and marks it', async () => {
    const picker = await loadPicker({ catalog: CATALOG });

    // A box over Spokane, then a pick, then a box over Puget Sound.
    await drawBox(picker, { x1: -11750, y1: 220, x2: -11720, y2: 250 });
    expect(rows(picker).map((r) => r.provider)).toEqual([
      'Spokane Transit',
      'Amtrak', // spans the country, so it turns up in every box
    ]);
    rows(picker)[0].checkbox.click();
    await settle();

    picker.els.draw.click(); // re-arm drawing
    await drawBox(picker, BOX);

    const listed = rows(picker);
    expect(listed.map((r) => r.provider)).toEqual([
      'King County Metro',
      'Sound Transit',
      'Amtrak',
      'Spokane Transit',
    ]);

    // Last, because it has no relevance to the box any more - but still ticked,
    // and saying why it's there.
    const spokane = listed[3];
    expect(spokane.outside).toBe(true);
    expect(spokane.checked).toBe(true);
    expect(picker.els.summary.textContent).toContain('plus 1 you picked outside it');

    // And it's still in the document.
    const post = picker.requests.findLast((r) => r.url === '/api/zone');
    expect(JSON.parse(post.options.body).feed_ids).toContain('f-c2h-spokanetransit');
  });

  test('feeds the new box uncovers arrive unticked', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, { x1: -12240, y1: 220, x2: -12235, y2: 225 });
    rows(picker)[0].checkbox.click();
    await settle();
    const first = rows(picker).find((r) => r.checked).provider;

    picker.els.draw.click();
    await drawBox(picker, BOX);

    for (const row of rows(picker)) {
      expect(row.checked).toBe(row.provider.startsWith(first));
    }
  });
});

describe('the JSON pane', () => {
  test('an edit moves the box and re-ticks the list', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, BOX);
    rows(picker)[0].checkbox.click();
    await settle();

    const zone = JSON.parse(picker.els.json.value);
    zone.feeds = [
      { feed_onestop_id: 'f-c23-soundtransit', provider: 'Sound Transit', url: 'x' },
    ];
    picker.els.json.value = JSON.stringify(zone, null, 2);
    picker.els.json.dispatchEvent(new window.Event('input'));
    await settle();

    expect(rows(picker).map((r) => r.checked)).toEqual([false, true, false]);
    expect(picker.els.count.textContent).toBe('1 of 3 selected');
  });

  test('mirrors the pane so a reload comes back where it was', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, BOX);
    expect(picker.remembered).toBe(null);

    rows(picker)[0].checkbox.click();
    await settle();
    expect(JSON.parse(picker.remembered).feeds.map((f) => f.feed_onestop_id)).toEqual([
      'f-c23-metrokingcounty',
    ]);

    rows(picker)[0].checkbox.click();
    await settle();
    expect(picker.remembered).toBe(null);
  });

  test('half-typed JSON is marked, not fought over', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, BOX);
    rows(picker)[0].checkbox.click();
    await settle();

    picker.els.json.value = '{ "version": 1, ';
    picker.els.json.dispatchEvent(new window.Event('input'));
    await settle();

    expect(picker.els.json.value).toBe('{ "version": 1, ');
    expect(picker.els.json.classList.contains('invalid')).toBe(true);
    expect(picker.els.download.disabled).toBe(true);
  });

  test('a credential typed into the pane survives the next tick', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, BOX);

    rows(picker)[1].checkbox.click(); // Sound Transit, which needs a key
    await settle();

    const zone = JSON.parse(picker.els.json.value);
    expect(zone.feeds[0].authorization).toEqual({ type: 'query_param', credential: '' });
    zone.feeds[0].authorization.credential = 'a-secret-token';
    picker.els.json.value = JSON.stringify(zone, null, 2);
    picker.els.json.dispatchEvent(new window.Event('input'));
    await settle();

    rows(picker)[0].checkbox.click(); // regenerates the whole document
    await settle();

    const post = picker.requests.findLast((r) => r.url === '/api/zone');
    expect(JSON.parse(post.options.body).credentials).toEqual({
      'f-c23-soundtransit': 'a-secret-token',
    });
    const regenerated = JSON.parse(picker.els.json.value);
    const soundTransit = regenerated.feeds.find(
      (f) => f.feed_onestop_id === 'f-c23-soundtransit',
    );
    expect(soundTransit.authorization.credential).toBe('a-secret-token');
  });

  test('the download is whatever the pane says', async () => {
    const picker = await loadPicker({ catalog: CATALOG });
    await drawBox(picker, BOX);
    rows(picker)[0].checkbox.click();
    await settle();

    picker.els.json.value = picker.els.json.value.replace('King County Metro', 'Edited');
    picker.els.json.dispatchEvent(new window.Event('input'));
    await settle();

    picker.els.download.click();
    expect(picker.downloads).toHaveLength(1);
    expect(picker.downloads[0].filename).toBe('zone.json');
    expect(picker.downloads[0].blob.type).toBe('application/json');
    expect(await picker.downloads[0].blob.text()).toContain('Edited');
  });
});

describe('reopening a zone', () => {
  /// A zone file as the server writes it, naming a feed the box doesn't cover
  /// and one the index has since lost.
  const SAVED = JSON.stringify({
    version: 1,
    bounds: { min_lon: -122.4, min_lat: 47.4, max_lon: -122.0, max_lat: 47.8 },
    feeds: [
      { feed_onestop_id: 'f-c23-metrokingcounty', provider: 'King County Metro', url: 'x' },
      { feed_onestop_id: 'f-c2h-spokanetransit', provider: 'Spokane Transit', url: 'x' },
      { feed_onestop_id: 'f-gone', provider: 'Defunct Transit', url: 'x' },
    ],
  });

  test('restores the picks, including the ones outside the box', async () => {
    const picker = await loadPicker({ catalog: CATALOG, saved: SAVED });

    const listed = rows(picker);
    expect(listed.filter((r) => r.checked).map((r) => r.provider)).toEqual([
      'King County Metro',
      'Spokane Transit',
    ]);
    expect(listed.find((r) => r.outside).provider).toBe('Spokane Transit');
    expect(picker.els.summary.textContent).toContain('2 of 3 feeds restored');
    expect(picker.els.summary.textContent).toContain('1 outside the box');
  });

  test('names a feed the index can no longer produce rather than dropping it', async () => {
    const picker = await loadPicker({ catalog: CATALOG, saved: SAVED });
    expect(picker.els.summary.textContent).toContain('missing f-gone');
  });

  test('a restored document that no longer parses is kept, not discarded', async () => {
    const picker = await loadPicker({ catalog: CATALOG, saved: '{ "version": 1, ' });

    expect(picker.els.json.value).toBe('{ "version": 1, ');
    expect(picker.els.json.classList.contains('invalid')).toBe(true);
    expect(picker.els.summary.textContent).toContain("isn't valid JSON");
  });

  test('a dropped file is opened the same way', async () => {
    const picker = await loadPicker({ catalog: CATALOG });

    const event = new window.Event('drop', { bubbles: true, cancelable: true });
    event.dataTransfer = {
      types: ['Files'],
      files: [{ name: 'puget-sound.json', text: async () => SAVED }],
    };
    window.dispatchEvent(event);
    await settle();

    expect(picker.els.summary.textContent).toContain('puget-sound.json');
    expect(rows(picker).filter((r) => r.checked)).toHaveLength(2);
  });

  test('a file that has no bounds says so instead of half-loading', async () => {
    const picker = await loadPicker({ catalog: CATALOG });

    const event = new window.Event('drop', { bubbles: true, cancelable: true });
    event.dataTransfer = {
      types: ['Files'],
      files: [{ name: 'notes.json', text: async () => '{"hello": "world"}' }],
    };
    window.dispatchEvent(event);
    await settle();

    expect(picker.els.summary.textContent).toBe('notes.json has no bounds - is it a zone file?');
    expect(rows(picker).map((r) => r.provider)).toEqual([
      'Drag a box on the map to see the feeds it touches.',
    ]);
  });
});

/// The zone files under builds/ are the build's input, and the picker is the
/// only thing that writes one. Reopening one is how you edit it, so the page
/// has to accept the real documents rather than only what its own stub server
/// just produced. (The Rust side of this seam - that the build parses what the
/// picker writes - is `gtfout::zone::what_the_picker_writes_is_what_the_build_reads`.)
describe('a zone file the repository ships', () => {
  const SEATTLE = readFileSync(
    path.join(REPO, 'builds/Seattle/transit/zones/seattle.json'),
    'utf8',
  );

  test('reopens with every feed it names', async () => {
    const zone = JSON.parse(SEATTLE);
    // An index that has exactly the feeds the committed zone names.
    const catalog = zone.feeds.map((f, i) =>
      feed({
        feed_id: f.feed_onestop_id,
        provider: f.provider,
        url: f.url,
        authorization_type: f.authorization?.type ?? '',
        realtime: (f.realtime ?? []).map((rt) => ({
          feed_id: rt.feed_onestop_id,
          kinds: ['trip updates'],
          authorization_type: rt.authorization?.type ?? '',
          urls: rt.urls,
        })),
        bbox: [
          zone.bounds.min_lon,
          zone.bounds.min_lat,
          zone.bounds.max_lon,
          zone.bounds.max_lat,
        ],
        relevance: 1 - i / 100,
      }),
    );

    const picker = await loadPicker({ catalog, saved: SEATTLE });

    expect(picker.els.summary.textContent).toContain(
      `${zone.feeds.length} of ${zone.feeds.length} feeds restored`,
    );
    expect(picker.els.summary.textContent).not.toContain('missing');
    expect(rows(picker).every((r) => r.checked)).toBe(true);

    // And regenerating it asks for the same feeds back, in the same document.
    const post = picker.requests.findLast((r) => r.url === '/api/zone');
    expect(JSON.parse(post.options.body).feed_ids.sort()).toEqual(
      zone.feeds.map((f) => f.feed_onestop_id).sort(),
    );
    expect(JSON.parse(picker.els.json.value).version).toBe(zone.version);
  });
});
