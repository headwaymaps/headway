// Loads the picker page - the real asset the Rust binary serves - into jsdom.
//
// The page is a single `<script type="module">` inside the HTML, so there is
// nothing to import directly. Rather than keeping a second copy of it that
// could drift, the harness lifts that script out of the file the server
// embeds, points its one CDN import at a local stub, and runs it.

import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const PAGE = path.join(here, '..', 'assets', 'zone-builder-server', 'index.html');
const STUB = pathToFileURL(path.join(here, 'maplibre-stub.mjs')).href;
const TMP = path.join(here, '.tmp');

/// Which style URL the harness substitutes for the server's `__MAP_STYLE__`.
export const MAP_STYLE = 'http://localhost:8000/style.json';

let generation = 0;

function pageSource() {
  const html = readFileSync(PAGE, 'utf8');
  const match = html.match(/<script type="module">([\s\S]*?)<\/script>/);
  if (!match) throw new Error(`no module script in ${PAGE}`);
  return { html, script: match[1], body: html.replace(match[0], '') };
}

/// A feed as `/api/feeds-by-bbox` returns it. Defaults are the boring case:
/// city-sized, no credential, no realtime.
export function feed(overrides = {}) {
  return {
    feed_id: 'f-test',
    provider: 'Test Transit',
    url: 'https://example.com/gtfs.zip',
    authorization_type: '',
    info_url: null,
    realtime: [],
    bbox: [-122.4, 47.4, -122.0, 47.8],
    area_m2: 1_000e6,
    relevance: 0.5,
    ...overrides,
  };
}

/// The zone document `/api/zone` returns, assembled the way the server does.
/// Mirrors `gtfout::zone::assemble` closely enough that the page's half of the
/// round trip - harvesting credentials back out of the pane - is exercised.
export function assembleZone({ bbox, feed_ids, credentials = {} }, catalog) {
  const [min_lon, min_lat, max_lon, max_lat] = bbox.split(',').map(Number);
  return {
    version: 1,
    bounds: { min_lon, min_lat, max_lon, max_lat },
    feeds: feed_ids.map((id) => {
      const known = catalog.find((f) => f.feed_id === id) || feed({ feed_id: id });
      const entry = {
        feed_onestop_id: id,
        provider: known.provider,
        url: known.url,
      };
      if (known.authorization_type) {
        entry.authorization = {
          type: known.authorization_type,
          credential: credentials[id] ?? '',
        };
      }
      if (known.realtime.length) {
        entry.realtime = known.realtime.map((rt) => {
          const out = { feed_onestop_id: rt.feed_id, urls: rt.urls ?? {} };
          if (rt.authorization_type) {
            out.authorization = {
              type: rt.authorization_type,
              credential: credentials[rt.feed_id] ?? '',
            };
          }
          return out;
        });
      }
      return entry;
    }),
  };
}

/// Boots the page against a stub server.
///
/// `catalog` is every feed the stub index knows about; a bbox query returns the
/// ones whose extent intersects it, which is what the real endpoint does.
export async function loadPicker({ catalog = [], saved = null } = {}) {
  const { script, body } = pageSource();

  document.documentElement.innerHTML = body.replace('__MAP_STYLE__', MAP_STYLE);

  // jsdom under Node 26 hands back the runtime's own localStorage, which is
  // off unless node was started with --localstorage-file. A plain map is all
  // the page needs, and it lets a test read back what was mirrored.
  const store = new Map();
  const localStorage = {
    getItem: (key) => (store.has(key) ? store.get(key) : null),
    setItem: (key, value) => store.set(key, String(value)),
    removeItem: (key) => store.delete(key),
    clear: () => store.clear(),
  };
  Object.defineProperty(window, 'localStorage', {
    value: localStorage,
    configurable: true,
  });
  if (saved !== null) localStorage.setItem('zone-builder-server.zone', saved);

  const requests = [];
  let zoneResponse = null;

  window.fetch = async (url, options = {}) => {
    requests.push({ url, options });

    if (url.startsWith('/api/feeds-by-bbox')) {
      const bbox = new URL(url, 'http://test').searchParams.get('bbox');
      const [w, s, e, n] = bbox.split(',').map(Number);
      const hits = catalog.filter(
        (f) => f.bbox[0] <= e && f.bbox[2] >= w && f.bbox[1] <= n && f.bbox[3] >= s,
      );
      return jsonResponse(hits);
    }

    if (url.startsWith('/api/feeds/')) {
      const ids = decodeURIComponent(url.slice('/api/feeds/'.length)).split(',');
      const found = ids
        .map((id) => catalog.find((f) => f.feed_id === id))
        .filter(Boolean)
        // No drawn area to rank against, so the endpoint reports no relevance.
        .map((f) => ({ ...f, relevance: null }));
      return jsonResponse(found);
    }

    if (url === '/api/zone') {
      const request = JSON.parse(options.body);
      zoneResponse = assembleZone(request, catalog);
      return textResponse(JSON.stringify(zoneResponse, null, 2) + '\n');
    }

    throw new Error(`unstubbed request: ${url}`);
  };

  // Downloading is an object URL on a synthetic <a> that the page clicks.
  // jsdom has neither - and clicking a link with an href it can't navigate to
  // logs a "not implemented" every time - so both halves are captured here.
  const blobs = [];
  window.URL.createObjectURL = (blob) => {
    blobs.push(blob);
    return 'blob:zone';
  };
  window.URL.revokeObjectURL = () => {};

  const downloads = [];
  const anchorClick = window.HTMLAnchorElement.prototype.click;
  window.HTMLAnchorElement.prototype.click = function () {
    if (!this.download) return anchorClick.call(this);
    downloads.push({ filename: this.download, blob: blobs[blobs.length - 1] });
  };

  mkdirSync(TMP, { recursive: true });
  const modulePath = path.join(TMP, `picker-${++generation}.mjs`);
  writeFileSync(
    modulePath,
    script.replace(/from '[^']*maplibre-gl[^']*'/, `from '${STUB}'`),
  );
  await import(pathToFileURL(modulePath).href);

  // The module's top-level `restoreSaved()` may still have queries in flight.
  await settle();

  return {
    requests,
    downloads,
    /// What the page mirrored to storage, so a reload comes back where it was.
    get remembered() {
      return localStorage.getItem('zone-builder-server.zone');
    },
    /// The last document `/api/zone` produced, as the pane should be showing it.
    get zone() {
      return zoneResponse;
    },
    els: {
      feeds: document.getElementById('feeds'),
      summary: document.getElementById('summary'),
      count: document.getElementById('count'),
      json: document.getElementById('json'),
      download: document.getElementById('download'),
      draw: document.getElementById('draw'),
      box: document.getElementById('box'),
    },
    canvas: document.getElementById('map').lastElementChild,
  };
}

function jsonResponse(value) {
  return {
    ok: true,
    json: async () => value,
    text: async () => JSON.stringify(value),
  };
}

function textResponse(text) {
  return { ok: true, text: async () => text, json: async () => JSON.parse(text) };
}

/// Lets every already-resolved promise in the page's chains run.
///
/// The page fans out - a query, then a render, then the zone POST the render
/// kicks off - so one await isn't enough to reach a settled UI.
export async function settle(rounds = 10) {
  for (let i = 0; i < rounds; i++) await Promise.resolve();
  await new Promise((resolve) => setTimeout(resolve, 0));
  for (let i = 0; i < rounds; i++) await Promise.resolve();
}

/// Drags a rectangle on the map, in screen pixels, the way a person would.
export async function drawBox(picker, { x1, y1, x2, y2 }) {
  const mouse = (type, target, x, y) =>
    target.dispatchEvent(
      new window.MouseEvent(type, {
        bubbles: true,
        cancelable: true,
        button: 0,
        clientX: x,
        clientY: y,
      }),
    );

  mouse('mousedown', picker.canvas, x1, y1);
  mouse('mousemove', window, x2, y2);
  mouse('mouseup', window, x2, y2);
  await settle();
}

/// The rows the panel is showing, in order.
///
/// `provider` is just the name: the tags beside it ("outside the box", "needs
/// query_param") are appended into the same element, and each has its own flag
/// here so a test asserting on names doesn't have to spell them out.
export function rows(picker) {
  return [...picker.els.feeds.querySelectorAll('li')].map((li) => {
    const provider = li.querySelector('.provider');
    return {
      checked: li.querySelector('input')?.checked ?? null,
      provider: provider ? provider.firstChild.textContent : li.textContent,
      needs: li.querySelector('.provider .auth')?.textContent ?? null,
      realtime: li.querySelector('.rt')?.textContent ?? null,
      meta: li.querySelector('.meta')?.textContent ?? '',
      outside: !!li.querySelector('.outside'),
      checkbox: li.querySelector('input'),
    };
  });
}
