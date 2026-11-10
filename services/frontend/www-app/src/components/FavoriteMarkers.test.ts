// @vitest-environment jsdom
import { afterEach, expect, test, vi } from 'vitest';
import { createApp, nextTick, type App } from 'vue';
import { favorites, type Favorite } from 'src/models/Favorites';
import FavoriteMarkers from './FavoriteMarkers.vue';

const state = vi.hoisted(() => ({
  push: vi.fn(),
  resolve: undefined as undefined | (() => void),
}));
vi.mock('vue-router', () => ({ useRouter: () => ({ push: state.push }) }));
vi.mock('./BaseMap.vue', () => ({
  map: {},
  baseMapPromise: new Promise<void>((resolve) => {
    state.resolve = resolve;
  }),
}));
vi.mock('src/models/Favorites', async () => {
  const { ref } = await import('vue');
  return { favorites: ref([]) };
});
// Mock the map SDK boundary; Vue, DOM nodes, event dispatch and watchers are real.
const popups: FakePopup[] = [];
class FakePopup {
  content?: HTMLElement;
  coordinates?: [number, number];
  closed = false;
  onClose?: () => void;
  constructor() {
    popups.push(this);
  }
  setLngLat(value: [number, number]) {
    this.coordinates = value;
    return this;
  }
  setDOMContent(value: HTMLElement) {
    this.content = value;
    return this;
  }
  addTo() {
    document.body.appendChild(this.content!);
    return this;
  }
  on(_event: string, callback: () => void) {
    this.onClose = callback;
    return this;
  }
  remove() {
    if (!this.closed) {
      this.closed = true;
      this.content?.remove();
      this.onClose?.();
    }
    return this;
  }
}
vi.mock('maplibre-gl', () => ({
  Popup: class {
    constructor() {
      return new FakePopup();
    }
  },
  Marker: class {
    element: HTMLElement;
    constructor(options: { element: HTMLElement }) {
      this.element = options.element;
    }
    setLngLat() {
      return this;
    }
    addTo() {
      document.body.appendChild(this.element);
      return this;
    }
    remove() {
      this.element.remove();
    }
  },
}));
let app: App | undefined;
const favorite = (id = 'one', description = 'A quiet place'): Favorite => ({
  id,
  name: `Favorite ${id}`,
  description,
  placeId: `place:${id}`,
  coordinates: id === 'two' ? [-121, 47] : [-122, 47],
  createdAt: '2026-01-01T00:00:00Z',
});
async function mount(items = [favorite()]) {
  favorites.value = items;
  const host = document.createElement('div');
  document.body.appendChild(host);
  app = createApp(FavoriteMarkers);
  app.mount(host);
  state.resolve?.();
  await nextTick();
  await nextTick();
}
function click(index = 0) {
  (
    document.querySelectorAll('.favorite-map-star')[index] as HTMLButtonElement
  ).click();
}
function current() {
  return popups.at(-1)!;
}
afterEach(() => {
  app?.unmount();
  app = undefined;
  favorites.value = [];
  popups.length = 0;
  document.body.replaceChildren();
  vi.clearAllMocks();
});

test('click opens saved name and description at the star and navigates to details', async () => {
  await mount();
  expect(popups).toHaveLength(0);
  click();
  expect(current().content?.querySelector('strong')?.textContent).toBe(
    'Favorite one',
  );
  expect(current().content?.querySelector('p')?.textContent).toBe(
    'A quiet place',
  );
  expect(current().coordinates).toEqual([-122, 47]);
  expect(state.push).toHaveBeenCalledWith('/favorites/one');
});
test.each([0, 140, 141])(
  'description length %i is omitted or truncated correctly',
  async (length) => {
    await mount([favorite('one', 'a'.repeat(length))]);
    click();
    expect(current().content?.querySelector('p')?.textContent).toBe(
      length === 0 ? undefined : length > 140 ? '…' : 'a'.repeat(length),
    );
  },
);
test('truncation preserves Unicode code points', async () => {
  await mount([favorite('one', '🌟 '.repeat(71).trim())]);
  click();
  expect(current().content?.querySelector('p')?.textContent).toBe(
    '🌟 '.repeat(70).trimEnd() + '…',
  );
});
test('user text is rendered as text rather than HTML', async () => {
  const item = {
    ...favorite('one', '<img src=x onerror=alert(1)>'),
    name: '<script>alert(1)</script>',
  };
  await mount([item]);
  click();
  expect(current().content?.querySelector('strong')?.textContent).toBe(
    item.name,
  );
  expect(current().content?.querySelector('p')?.textContent).toBe(
    item.description,
  );
  expect(current().content?.querySelector('script, img')).toBeNull();
});
test('clicking another star closes the previous popup', async () => {
  await mount([favorite(), favorite('two')]);
  click();
  const first = current();
  click(1);
  expect(first.closed).toBe(true);
  expect(current().content?.textContent).toContain('Favorite two');
  expect(popups.filter((popup) => !popup.closed)).toHaveLength(1);
});
test('saving edits refreshes the open popup', async () => {
  await mount();
  click();
  favorites.value = [
    { ...favorite(), name: 'Renamed', description: 'Updated notes' },
  ];
  await nextTick();
  expect(current().content?.textContent).toBe('RenamedUpdated notes');
  expect(popups.filter((popup) => !popup.closed)).toHaveLength(1);
});
test('removing the selected favorite removes its popup and star', async () => {
  await mount();
  click();
  const popup = current();
  favorites.value = [];
  await nextTick();
  expect(popup.closed).toBe(true);
  expect(document.querySelector('.favorite-map-star')).toBeNull();
  expect(document.querySelector('.favorite-map-preview')).toBeNull();
});
test('a dismissed popup stays closed when favorites change', async () => {
  await mount();
  click();
  current().remove();
  favorites.value = [favorite(), favorite('two')];
  await nextTick();
  expect(popups).toHaveLength(1);
  expect(document.querySelector('.favorite-map-preview')).toBeNull();
});
test('unmount cleans up popup and markers', async () => {
  await mount();
  click();
  const popup = current();
  app?.unmount();
  app = undefined;
  expect(popup.closed).toBe(true);
  expect(
    document.querySelector('.favorite-map-star, .favorite-map-preview'),
  ).toBeNull();
});

test('long bubble names truncate at a whole word under 50 characters', async () => {
  const prefix = 'Nice '.repeat(9).trimEnd();
  await mount([{ ...favorite(), name: `${prefix} wonderful cafe` }]);
  click();
  expect(current().content?.querySelector('strong')?.textContent).toBe(
    `${prefix}…`,
  );
});

test.each([2, 3, 8])(
  'one marker opens a chooser for %i colocated favorites',
  async (count) => {
    const items = Array.from({ length: count }, (_, index) =>
      favorite(`business-${index}`),
    );
    await mount(items);
    expect(document.querySelectorAll('.favorite-map-star')).toHaveLength(1);
    expect(
      document.querySelector('.favorite-map-star')?.getAttribute('aria-label'),
    ).toBe(`${count} favorites here`);
    click();
    expect(current().content?.querySelector('strong')?.textContent).toBe(
      `${count} favorites here`,
    );
    const choices = current().content!.querySelectorAll('button');
    expect(choices).toHaveLength(count);
    expect(state.push).not.toHaveBeenCalled();
    (choices[count - 1] as HTMLButtonElement).click();
    expect(state.push).toHaveBeenCalledWith(`/favorites/business-${count - 1}`);
  },
);
test('group count updates after deletion and becomes a normal star', async () => {
  const items = [favorite('a'), favorite('b'), favorite('c')];
  await mount(items);
  click();
  favorites.value = items.slice(1);
  await nextTick();
  expect(current().content?.querySelector('strong')?.textContent).toBe(
    '2 favorites here',
  );
  favorites.value = items.slice(2);
  await nextTick();
  expect(current().content?.querySelector('strong')?.textContent).toBe(
    'Favorite c',
  );
  expect(document.querySelector('.favorite-map-count')).toBeNull();
  click();
  expect(state.push).toHaveBeenCalledWith('/favorites/c');
});
test('different coordinates retain separate stars', async () => {
  await mount([favorite('one'), favorite('two')]);
  expect(document.querySelectorAll('.favorite-map-star')).toHaveLength(2);
});
