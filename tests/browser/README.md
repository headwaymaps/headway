# Browser Tests

Playwright UI tests that drive the Headway frontend in real Google Chrome
against a running stack.

These complement `tests/integration/*.sh`. Those assert that the services return
correct data over HTTP; these assert the app actually wires that data into a
usable map — tiles painted to the canvas, panning and zooming, the geocoder
menu, and both routing engines.

Like the integration tests, they assert against the locations in a build's
`smoke-test.env`, e.g. `builds/Bogota/smoke-test.env`.

## Running

Start a stack first, exactly as for the integration tests:

```bash
bin/start-services --no-follow-logs builds/Bogota
bin/wait-for-services
```

Then:

```bash
bin/test/browser-tests builds/Bogota                # headless (installs deps on first run)
bin/test/browser-tests builds/Bogota --headed       # watch it drive
bin/test/browser-tests builds/Bogota -g "routing"   # a subset
```

`bin/test/browser-tests` checks the stack is actually up, loads the build's `smoke-test.env`,
and passes its remaining arguments through to `playwright test`. To drive
Playwright directly instead:

```bash
cd tests/browser
yarn install          # first time
set -a; source ../../builds/Bogota/smoke-test.env; set +a
npx playwright test
npx playwright show-report       # last HTML report
```

Point at a different stack with `HEADWAY_PUBLIC_URL=http://host:port`.

Failures keep a screenshot, video and trace under `test-results/`. Open a trace
with `npx playwright show-trace test-results/<dir>/trace.zip`.

## What's covered

| Test | Flow |
| --- | --- |
| map renders vector tiles | tiles fetched from `/tileserver`, all non-error, canvas actually painted |
| map can be dragged and zoomed | pointer-drag pans the camera, `+` control raises the zoom level |
| geocoder autocompletes | typing a landmark populates the suggestion menu |
| selecting a result | clicking a suggestion opens that place's page |
| search deep link | `/search/:text` renders matching places |
| driving directions | place -> Drive -> pick an origin -> Valhalla returns a route with a duration |
| transit directions | place -> Transit -> pick an origin -> OpenTripPlanner returns an itinerary |

## Notes for editing these tests

- **Asserting the map moved.** `BaseMap.vue` persists the camera to
  `localStorage` (`mostRecentMapCenter` / `mostRecentMapZoom`) on `moveend`,
  **debounced by 2s**. The tests clear those keys, interact, then poll for them
  to reappear. That is a real assertion that the camera moved, and it is why
  they poll rather than reading straight back. Don't tighten those waits below
  the debounce.

- **Place names are not the query.** Many POIs carry an English `name:en` that
  the app prefers, so searching Bogota's "Museo del Oro" lands on a page titled
  "Gold Museum". Fixtures are chosen so the display name matches the query, and
  so the query's top result is right even without a focus point - on the planet
  a bare "Monserrate" ranks a place in Portugal first.

- **Quasar class names are BEM.** Labels are `.q-item__label`, not
  `.q-item-label`. The list items themselves are `.q-item`, and trip rows carry
  `.list-item`.

- **Transit may legitimately find nothing.** The transit test accepts either an
  itinerary or the app's "no route" message; only an unhandled failure fails it.
