# maplibre-style

`src/headway.rs` is the source of truth for the Headway MapLibre style.
`headway-style` renders it as JSON; tile-server image builds place that output
at `assets/styles/basic-v3.json`.

For local map iteration, run `bin/dev-local-assets`. Its
`HEADWAY_LOCAL_STYLE` mode rerenders the style when `maplibre-style/src` changes and
reloads the browser.

`tests/fixtures/headway.json` is the expected rendered style. The tests compare
it byte-for-byte and semantically, allowing only harmless formatting and legacy
filter spelling differences. Regenerate it from this directory with:

```sh
cargo run --bin headway-style > tests/fixtures/headway.json
```
