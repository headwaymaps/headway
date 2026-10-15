# mlss

Builders for generating MapLibre style documents.

`src/headway.rs` is the Headway default style and its single source of truth.
`headway-style` prints it as JSON. Tile-server image builds render it into
`assets/styles/basic-v3.json` in the image. For local map iteration, run
`bin/dev-local-assets`; its `HEADWAY_LOCAL_STYLE` mode rebuilds the style when
files under `mlss/src` change and reloads the browser.

## The pieces

- `Style` holds sources and layers. `style.source(..)` hands back a `SourceRef`,
  and `source.layer("transportation")` a `SourceLayer`. `OpenMapTiles` names the
  schema's source layers, e.g. `openmaptiles.transportation()`.
- `style.fill(&source_layer, "id")`, `.line(..)`, and `.symbol(..)` start a layer
  that is appended to the style when its builder statement ends, so layer order
  is call order. `add_layers` does the same for several layers of one source,
  and `line_factory` / `symbol_factory` share properties across layers.
- Layers only expose the paint and layout properties their type has.
  `.paint(name, value)` and `.layout(name, value)` cover anything not yet wrapped.
- `zoom!(base: 1.2; 12 => 0.5, 20 => 10.0)` is a zoom function, written as the
  table it is. Any property takes either a constant or one of these.
- `expr![Match, [Get, "class"], "park", "#0a0", "transparent"]` is a MapLibre
  expression.
- `hsl!`, `rgb!` and friends write colors in the notation they're authored in.
- `Filter` builds the legacy filter syntax the openmaptiles styles use.

```rust
style
    .line(&roads, "motorway")
    .filter(Filter::class("motorway"))
    .rounded()
    .color(rgb!(255, 204, 136))
    .width(zoom!(base: 1.2; 7 => 1.0, 20 => 18.0));
```

## Tests

`tests/fixtures/headway.json` is the expected output, checked two ways:

- byte for byte, which catches any change at all;
- semantically, ignoring formatting and clause order.

If only the byte-for-byte test fails, the change is formatting; regenerate the
fixture with `cargo run -p mlss --bin headway-style > tests/fixtures/headway.json`.
