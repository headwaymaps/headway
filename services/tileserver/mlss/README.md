# mlss

Builders for generating MapLibre style documents.

A style sheet is a very verbose JSON document with a lot of near-repetition:
the road layers alone are the same handful of lines repeated across tunnel,
surface and bridge, each with a casing underneath, each fading in and widening
as you zoom. This crate lets that be written once.

`headway-style` prints the generated document for inspection. Tile-server image
builds render it automatically, replacing `assets/styles/basic-v3.json` in the
image with the current Rust implementation. For local map iteration, run
`bin/dev-local-assets`; its `HEADWAY_LOCAL_STYLE` mode rebuilds the style when
files under `mlss/src` change and reloads the browser.

`src/headway.rs` is the Headway default style, and
`tests/basic_v3.rs` asserts the builder-generated document reproduces the hand-written
`assets/styles/basic-v3.json` layer for layer. The comparison ignores two
things maplibre doesn't care about: integers written as floats, and the order
of clauses inside an `all` filter.

`bin/count-headway-style` counts the root declaration plus every private
Headway helper module under `src/headway/`; it cannot be improved merely by
moving Headway-specific code out of `headway.rs`.

## The pieces

- `Style` holds sources and layers. `style.source(..)` hands back a `SourceRef`,
  `source.layer("transportation")` a `SourceLayer`, and layers are built from that.
- `Layer<Line>`, `Layer<Fill>`, `Layer<Symbol>` … only expose the paint and layout
  properties their type actually has. `.paint(name, value)` and `.layout(name, value)`
  are there for anything not yet wrapped.
- `zoom!(base 1.2; 12 => 0.5, 20 => 10.0)` is a zoom function, written as the table
  it is. Any property takes either a constant or one of these.
- `style.fill(source, "id")`, `style.line(source, "id")`, and
  `style.symbol(source, "id")` attach a layer at the end of the builder chain.
- `Color` does arithmetic — `.darken(0.1)`, `.mix(&other, 0.5)`, `.alpha(0.8)` — and
  renders back in the notation it was written in until you change it.
- `Filter` builds the legacy filter syntax the openmaptiles styles use.
- `Streets` collects road layers and keeps casings in a tier below every surface,
  so a casing never paints over the neighbouring street at an intersection.

```rust
let mut streets = Streets::new();
let asphalt = Color::parse("#fc8");
streets.cased(
    roads.line("motorway").color(asphalt.clone()).width(width),
    1.0,
    asphalt.darken(0.15),
);
style.extend(streets);
```

`cased` derives the casing from the street: same filter and id with `_casing`,
widened by the pad at every zoom stop, undashed. Where a style wants a casing
that isn't a widened copy — osm-liberty's never are — build it yourself and hand
it to `Streets::casing`.

## Roads, three times over

`headway.rs` draws the road network three times: buried, at grade, and raised.
`Level` names which copy a layer belongs to, and `Level::pick(tunnel, surface,
bridge)` supplies whatever differs between them. The `brunnel` tag itself is only
set on the two copies that aren't at grade. A road class is then one declaration,
not one per level:

```rust
self.road("motorway_link", links, "#fc8", ramp)
    .cased(MAJOR_CASING, ramp_casing)
    .dashed_when_buried()
    .appears_at(12.0)
    .push(&mut streets);
```

## Expressive comparison

`../expressive` is a pinned checkout of
[OpenFreeMap Bright in Expressive](https://github.com/falseinput/openfreemap-expressive),
a different authoring language for MapLibre styles. It keeps parameters,
derived tokens, and layer modules in separate files. Run `bin/test-expressive-style`
from the repository root to compile both the upstream 119-layer example and
`expressive/headway-v3.exp`. The latter is a complete Headway v3 Expressive
implementation, split into ordered layer modules and checked for exact output
parity.
