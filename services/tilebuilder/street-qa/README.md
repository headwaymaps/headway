# Real-width streets

Parked October 2026. At high zoom, streets are drawn as the ground they cover, at their real width in meters, with
sidewalks along their curbs, instead of as lines. It looks good across most of Seattle and is not ready to deploy. This
branch (`mkirk/street-widths`) sits on `mkirk/street-profile`, which ships the simpler street changes (roundabouts,
crosswalks, unmarked crossings) without any of this.

## How it works

Everything is worked out in planetiler at z15. The style only fills polygons.

1. **Width tags** (`HeadwayProfile.tagWidth`). Each street gets `width` in meters: as mapped, or else lanes × 3.25 m.
   Streets with neither get a default for their class in `RoadSurfaces.width`.
2. **Tapered widths** (`WidthRanges`). Pieces of one street that differ only in width are joined into one line, with
   its width as `start,end,width` ranges along it, so it eases from one width to the next (6 m of taper per meter of
   change). Streets that meet end to end ease to the average width at the joint. A dual carriageway's two oneway halves
   widen where they run close, so together they span the whole roadway, and hold to the width of the single line they
   split from. Ranges are knotted wherever the eased curve would miss a sampled width.
3. **Street surfaces** (`RoadSurfaces`, a tile post-processor at z15):
   - Each street is buffered to its tapered widths (JTS `VariableBuffer`).
   - Streets are grouped by grade (tunnel, ground, bridge), so a bridge never merges with the street under it.
   - Within a grade, each kind (street class plus ramp) is closed (min(5 m, 0.4 × class width)), pointed fills are
     dropped, then it's opened by 1 m. Each kind also gets 20 m stubs of other kinds that meet it, so corners between
     kinds close.
   - The kinds are unioned into one surface per grade. Then each junction's inside corners are rounded, with a radius of
     min(5 m, 0.4 × the narrower street there), locally, so gaps between nearby streets stay open.
   - The surface is VW-simplified and snapped to the tile's 4096 grid. One `street_surface` feature per grade.
4. **Sidewalks**. Each mapped sidewalk is matched to the curb it runs beside, then drawn flush along that curb at a
   uniform 2 m, following the curb around corners. Neighbors are unioned with the street and rounded by 3 m, then the
   street is cut back out. Mapped sidewalk positions and widths are too unreliable to draw faithfully.
5. **Style** (`maplibre-style`). Below z15.5, lines widen toward their width in meters. From z15.5 the surfaces fade in.
   Class colors fade into one street grey from z14 to z16 (`TRUE_TO_SCALE_ZOOM`). Streets come from their own
   `streets` MLT source.

## Resuming

Run these with `services/tilebuilder/street-qa` as the working directory:

- `./build-streets.sh <repo>/data/Seattle/Seattle.osm.pbf <out-dir>` builds this checkout's streets (about 4 min for
  Seattle; it also runs the tests). It prints `errors: N`, the tiles that failed to post-process. N should be 0.
- Edit `martin.yaml`, then run martin on :8098, `node qa-proxy.mjs` (:8099), `node case-status.mjs` (:8091), and
  `HEADWAY_HOST=http://127.0.0.1:8099 bin/dev-local-assets` with the compose stack up on :8080.
- Open <http://localhost:9000/dev/street-qa>. Each case has a map view and a verdict: 👍, or 👎 with a note. Verdicts,
  notes, added cases, and Claude's replies are kept in `cases.json`, so commit it.
- `node screenshot.mjs <out> <zoom/lat/lng>...` screenshots views from the dev server.

## What kept going wrong

- **Morphology deciding meaning.** Closing (grow, then shrink) is used to fill gaps between widened lines, and any gap
  narrower than twice its radius might be a median, an island, a driveway mouth, or an error. Every radius traded one
  artifact for another: bumps between unrelated parallel streets, paved islands, notches at driveways, spikes where
  streets meet at sharp angles. Closing per kind and then rounding only near junctions made it tractable, not robust.
- **Dual carriageways.** Where a street splits into oneway halves, two independent buffers have to stand in for one
  changing roadway. That caused bulges where the halves rejoin, pinches, and curb steps (Alaskan Way, Denny, Montlake,
  E Lake Washington Blvd). Most of the `WidthRanges` complexity is here.
- **Tile seams.** Each tile builds its streets from input clipped at its own edges, so the same street can come out a
  grid point different in two tiles. The E James St seam went away as the surface code settled. A full fix was tried:
  shared working grid, unsimplified buffers, wider densified buffers, a local split hold, and quantized scale
  (`seams-experiment.patch`). Ported onto this branch, the fixes that didn't change the map's look made the build 2.5×
  slower and tiles 16% larger, broke 4 tiles, and changed nothing visible. Quantizing the scale is wrong in principle:
  it makes latitude bands with bigger seams at their edges. Scale should come from each point's own latitude.
- **Sidewalks vs. curbs.** Sidewalks are mapped as centerlines, often metres from the curb, and are independent of it.
  Rays from the line to the curb weren't parallel to it, corners weren't concentric with the curb, and drawing from the
  curb to the line paved planting strips 4–6 m wide. Settled by ignoring where they're mapped.
- **Brittle JTS.** Exact-precision overlays throw `TopologyException` on near-coincident buffer curves. A tile that
  throws renders without streets. Keep `OverlayNGRobust` and `GeometryFixer`, and only snap at the end.
- **Width tests are brittle.** `WidthRangesTest` compares exact range strings, so every taper or knot change rewrites
  expectations. Most changes needed a full build to judge, because the fast harness used pre-baked widths.

## Open QA cases

From `cases.json`, voted 👎:

- 7th Ave crossing Denny into Wall St: the inner sidewalk should parallel the road's curve at a uniform width; a 0.2 m
  tile-edge step remains at z22.
- 13th Ave E & E Republican St, and an added "inner sidewalk corner should be curved": sidewalk corner radius should
  parallel the curb's. Revisit now that sidewalks are flush and uniform.
- Union & 25th: a missing curb radius. Also there: a crossing that looks unmarked is drawn.
- Aurora north junction: a gap in the road.
- 17th Ave @ Mercer: sidewalk too wide. Likely fixed by the uniform 2 m sidewalk.

## Before it can ship

- **Build and serve the streets.** The style reads a separate `streets` source (transportation to z15, with
  `street_surface` and sidewalks). Only `build-streets.sh` produces it. The dagger build and the tileserver need to
  build and serve it, or fold the layers into the main tiles.
- **Planet cost.** Seattle's streets take about 4 minutes. Measure a large region before committing to a planet build.
  Watch the error count and tile size too.
- **iOS.** The style change has to work in MapLibre Native too.

## Thoughts for a successful conclusion

- **Keep the scope to what looks right.** Faithful geometry from OSM isn't achievable here; uniform defaults that look
  good (as with sidewalks) beat mapped data that's wrong half the time.
- **Replace morphology at junctions with explicit geometry.** Most remaining artifacts happen where closing decides
  what a junction should look like. A small junction model fixes them by construction: trim each street back to where
  its curbs meet the next street's, then fill the junction as one polygon with corners of a chosen radius. This is the
  osm2streets / A/B Street approach. Borrow the idea and its catalogue of pathological cases, not the library or its
  lane model. Start with ordinary 3–4 way junctions, which are most of the map. Keep the current union-and-close as the
  fallback for everything else.
- **Make divided roads a graph step.** Merging a dual carriageway's halves into one wide street, before any geometry,
  would remove the split/merge bulges and most of `WidthRanges`.
- **Seams only if they show.** Fix seams only if QA finds a visible one. Then compute scale per point and make every
  step depend only on geometry within a radius the tile buffer covers. Measure build time and size alongside.
- **Faster feedback.** A harness that renders one tile's surfaces to SVG straight from OSM input would replace
  4-minute builds for most iterations. Golden images of the QA cases would catch regressions better than exact
  range-string tests.
