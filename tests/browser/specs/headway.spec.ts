import { test, expect, type Page, type Locator } from '@playwright/test';

/**
 * UI tests for the Headway frontend, driven against a running stack (see
 * ../README.md). They complement tests/integration/*.sh, which exercise the
 * same services at the HTTP level: these assert that the app actually wires
 * those responses into a usable map.
 *
 * Fixtures are Bogota landmarks, matching builds/Bogota — the area the
 * integration tests assert against.
 */

const MAP_CANVAS = '#map canvas.maplibregl-canvas';

// Bogota fixtures. Both are chosen because OSM's display name matches the
// query: many Bogota POIs carry an English `name:en` that the app prefers, so
// e.g. searching "Museo del Oro" lands on a page titled "Gold Museum".
const DESTINATION = 'Universidad Nacional';
const ORIGIN = 'Monserrate';

/** Wait for MapLibre to mount its canvas at a non-zero size. */
async function waitForMap(page: Page): Promise<Locator> {
  const canvas = page.locator(MAP_CANVAS);
  await expect(canvas).toBeVisible();
  const box = await canvas.boundingBox();
  expect(box, 'map canvas should be laid out').not.toBeNull();
  expect(box!.width).toBeGreaterThan(100);
  return canvas;
}

/**
 * BaseMap.vue persists the camera to localStorage on `moveend`, debounced by
 * 2s. So clearing a key and waiting for it to come back is a direct assertion
 * that the map camera really moved — no pixel-diffing required. The debounce is
 * why these poll instead of reading straight back.
 */
async function clearStoredCamera(page: Page): Promise<void> {
  await page.evaluate(() => {
    localStorage.removeItem('mostRecentMapCenter');
    localStorage.removeItem('mostRecentMapZoom');
  });
}

async function waitForStoredCenter(page: Page): Promise<string> {
  const read = () => page.evaluate(() => localStorage.getItem('mostRecentMapCenter'));
  await expect
    .poll(read, { timeout: 30_000, message: 'map never recorded a new center' })
    .not.toBeNull();
  return (await read())!;
}

async function waitForStoredZoom(page: Page): Promise<number> {
  const read = () => page.evaluate(() => localStorage.getItem('mostRecentMapZoom'));
  await expect
    .poll(read, { timeout: 30_000, message: 'map never recorded a new zoom' })
    .not.toBeNull();
  return Number(JSON.parse((await read())!));
}

/** Drag across the map canvas. Stepped, so MapLibre pans instead of clicking. */
async function dragMap(page: Page, canvas: Locator, dx: number, dy: number): Promise<void> {
  const box = (await canvas.boundingBox())!;
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + dx, y + dy, { steps: 20 });
  await page.mouse.up();
}

/** Type into a search box and wait for the autocomplete menu to populate. */
async function typeSearch(page: Page, input: Locator, text: string): Promise<Locator> {
  await input.click();
  await input.fill(text);
  const suggestions = page.locator('.auto-complete-menu .q-item');
  await expect(suggestions.first()).toBeVisible();
  return suggestions;
}

/** Front page -> search -> pick the first hit, landing on that place's page. */
async function openPlacePage(page: Page, query: string): Promise<void> {
  await page.goto('/');
  await waitForMap(page);
  const suggestions = await typeSearch(page, page.locator('.search-box input').first(), query);
  await suggestions.first().click();
  await expect(page).toHaveURL(/\/place\//);
}

test('map renders vector tiles from the tileserver', async ({ page }) => {
  const tiles: { url: string; status: number }[] = [];
  page.on('response', (r) => {
    if (/\/tileserver\/.+\/\d+\/\d+\/\d+/.test(r.url())) {
      tiles.push({ url: r.url(), status: r.status() });
    }
  });

  await page.goto('/');
  const canvas = await waitForMap(page);

  await expect
    .poll(() => tiles.length, { message: 'no tile requests were made' })
    .toBeGreaterThan(0);
  expect(tiles.filter((t) => t.status >= 400), 'tile requests should all succeed').toEqual([]);

  // A blank/uniform canvas compresses to a very small PNG; a painted map of
  // roads and labels does not. Cheap proof that pixels actually landed.
  const painted = await canvas.screenshot();
  expect(painted.byteLength).toBeGreaterThan(10_000);
});

test('map can be dragged to a new location', async ({ page }) => {
  await page.goto('/');
  const canvas = await waitForMap(page);

  await clearStoredCamera(page);
  await dragMap(page, canvas, -250, -160);
  const afterFirstDrag = await waitForStoredCenter(page);

  await clearStoredCamera(page);
  await dragMap(page, canvas, 250, 160);
  const afterSecondDrag = await waitForStoredCenter(page);

  expect(afterSecondDrag, 'dragging back should land somewhere else').not.toEqual(afterFirstDrag);
});

test('map zoom controls change the zoom level', async ({ page }) => {
  await page.goto('/');
  await waitForMap(page);

  const zoomIn = page.locator('.maplibregl-ctrl-zoom-in');
  await expect(zoomIn).toBeVisible();

  await clearStoredCamera(page);
  await zoomIn.click();
  const before = await waitForStoredZoom(page);

  await clearStoredCamera(page);
  await zoomIn.click();
  const after = await waitForStoredZoom(page);

  expect(after).toBeGreaterThan(before);
});

test('geocoder autocompletes a landmark', async ({ page }) => {
  await page.goto('/');
  await waitForMap(page);

  const suggestions = await typeSearch(page, page.locator('.search-box input').first(), DESTINATION);
  await expect(suggestions.first()).toContainText(new RegExp(DESTINATION, 'i'));
});

test('selecting a search result opens its place page', async ({ page }) => {
  await page.goto('/');
  await waitForMap(page);

  const suggestions = await typeSearch(page, page.locator('.search-box input').first(), DESTINATION);
  // Carry the label through rather than re-asserting the query: the app prefers
  // OSM's `name:en`, so the result's title is not always the text we typed.
  // Each suggestion renders the place name on the first line and its address
  // on the second.
  const chosen = (await suggestions.first().innerText()).trim().split('\n')[0].trim();
  expect(chosen).not.toEqual('');

  await suggestions.first().click();
  await expect(page).toHaveURL(/\/place\//);
  await expect(page.locator('.bottom-card')).toContainText(chosen);
});

test('search deep link lists matching places', async ({ page }) => {
  await page.goto(`/search/${encodeURIComponent(ORIGIN)}`);
  await waitForMap(page);

  const results = page.locator('.bottom-card .search-results .q-item');
  await expect(results.first()).toBeVisible();
  await expect(results.first()).toContainText(new RegExp(ORIGIN, 'i'));
});

test('driving directions return a route with a duration', async ({ page }) => {
  await openPlacePage(page, DESTINATION);

  await page.locator('.travel-mode-bar').getByRole('link', { name: 'Drive' }).click();
  await expect(page).toHaveURL(/\/directions\/car\//);

  // tabindex 1 is the "from" box, 2 is "to" (see TripSearch.vue).
  const suggestions = await typeSearch(page, page.locator('input[tabindex="1"]'), ORIGIN);
  await suggestions.first().click();

  const trips = page.locator('.bottom-card .list-item');
  await expect(trips.first()).toBeVisible();
  // Valhalla's duration, formatted by the app, e.g. "12 min" / "1 hr 5 min".
  await expect(trips.first()).toContainText(/\d+\s*(min|hr)/i);
});

test('transit directions return an itinerary', async ({ page }) => {
  await openPlacePage(page, DESTINATION);

  await page.locator('.travel-mode-bar').getByRole('link', { name: 'Transit' }).click();
  await expect(page).toHaveURL(/\/directions\/transit\//);

  const suggestions = await typeSearch(page, page.locator('input[tabindex="1"]'), ORIGIN);
  await suggestions.first().click();

  // OpenTripPlanner is slower than Valhalla, and may legitimately report that
  // no itinerary exists; either is a working transit stack, an unhandled
  // failure is not.
  const trips = page.locator('.bottom-card .list-item');
  const error = page.locator('.search-error');
  await expect(trips.first().or(error)).toBeVisible({ timeout: 90_000 });

  if (await trips.first().isVisible()) {
    await expect(trips.first()).toContainText(/\d+\s*(min|hr)/i);
  }
});
