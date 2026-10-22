# maplibre-style

`src/headway/` is the source of truth for the Headway MapLibre style.
`headway-style v3|v4` renders a revision of it as JSON; tile-server image builds
place those at `assets/styles/basic-v3.json` and `basic-v4.json`. v4 draws
buildings in 3D, which v3 clients add themselves.

For local map iteration, run `bin/dev-local-assets`. Its
`HEADWAY_LOCAL_STYLE` mode rerenders the style when `maplibre-style/src` changes and
reloads the browser.

`tests/fixtures/headway.json` (v3) and `headway-v4.json` are the expected
rendered styles. The tests compare them byte-for-byte. Regenerate them from
this directory with:

```sh
cargo run --bin headway-style v3 > tests/fixtures/headway.json
cargo run --bin headway-style v4 > tests/fixtures/headway-v4.json
```
