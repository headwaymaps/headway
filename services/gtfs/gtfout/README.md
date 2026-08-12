# gtfout

Tools for working out which GTFS feeds a transit zone should be built from, and
for fetching them once you've decided.

The Transitland Atlas catalogues thousands of feeds but records no service area
for any of them — DMFR has no bounding box and no country field. So we work out
where each feed is by downloading it and reading its stops, once, into a
GeoPackage index. After that, "which feeds serve this area" is a spatial query.

That split is why there are two steps below: one slow and global, one instant and
repeatable.

## Quick start: the zone builder

The picker is a small admin web app for defining a transit zone: draw a box on a
map, tick the feeds it should include, download the zone file. It only reads the
index, so **you have to build the index first**.

### 1. Build the index

```
cd ../../..
cargo build --release --package gtfout

./target/release/write-gtfs-index --all \
  --atlas-path ./atlas --download \
  --out ./feed-extents.gpkg
```

`--download` clones the atlas to `./atlas` on the first run and refreshes it on
later ones; leave it off to use a clone you manage yourself.

This downloads every catalogued feed to measure it. **Budget around 15 minutes
and some GB of traffic.** You only pay it once — feeds already in the index are
never re-fetched, so re-running later only picks up what the catalog has gained.
Check the size of the job first with `--dry-run`:

```
$ ./target/release/write-gtfs-index --all --atlas-path ./atlas --out ./feed-extents.gpkg --dry-run
4069 gtfs feeds with a static_current url, 42 without
0 already measured, 4069 to fetch
--dry-run: stopping before downloading anything
```

Progress is reported as it goes, along with each feed that fails:

```
  failed f-9q9q-wheelsbus: HTTP status client error (404 Not Found) for url (…)
  1,204/4,069 feeds · 31 failed · 3.71 GB · 11.6 MB/s · 5m21s elapsed · ~12m04s left
```

Failures are worth reading rather than scrolling past. A feed that couldn't be
downloaded has no extent, so it matches no area — it will be silently missing
from every zone, and this is the one place that's cheap to notice.

### 2. (Optional) Supply credentials

Some agencies won't serve their feed without a key. Generate a template listing
the ones that need one:

```
./target/release/write-gtfs-index --all \
  --atlas-path ./atlas --out ./feed-extents.gpkg \
  --dry-run --write-config-template ./gtfs-credentials.yaml
```

Each entry is annotated with how the feed authenticates and where to request a
token:

```yaml
feeds:
  # San Francisco Municipal Transportation Agency
  # sent as query_param "api_key"
  # request one at: https://511.org/open-data/token
  "f-9q8y-sfmta": ""
```

Fill in what you have, delete the rest, then re-measure those feeds. Failures are
remembered so a dead server isn't retried every run, which means a newly supplied
credential has no effect without `--retry-failed`:

```
./target/release/write-gtfs-index --all \
  --atlas-path ./atlas --out ./feed-extents.gpkg \
  --config ./gtfs-credentials.yaml --retry-failed
```

To retry a single agency instead of sweeping the whole catalog, name it —
`--feed` always re-measures, even if the index already has an answer:

```
./target/release/write-gtfs-index --feed f-9q8y-sfmta \
  --atlas-path ./atlas --out ./feed-extents.gpkg --config ./gtfs-credentials.yaml
```

Regenerating the template later keeps whatever you've already filled in, so it's
safe to re-run over a working config.

### 3. Start the picker

```
bin/start-zone-builder-server
```

Then open <http://127.0.0.1:8420>.

The **Draw area** tool sits on the map, armed. Drag a box over your zone and it
disarms itself, so the map goes back to panning — draw again when you want a
different area, or press Esc to cancel without drawing.

The window is three columns: the map, the feed list, and the zone document. The
middle column lists every feed whose measured extent intersects the box, **the
best match first** — the operator whose service area looks most like the area
you drew. Feeds far larger than the box sink: a continental rail operator does
intersect your city, but it isn't what you're building a city zone out of, and
it scores near zero against one. Each row shows the feed's area in km², so you
can see why it placed where it did. Hovering a row outlines that feed on the
map; ticking it keeps the outline.

Nothing is ticked for you — drawing a box proposes candidates, it doesn't choose
them. Once ticked, though, a feed stays ticked: resizing the box is how you
explore, so a feed the box has moved off stays in the list, still selected and
marked "outside the box". Dropping it takes unticking it.

The right column is the zone file as it currently stands, and it's editable —
that's where you type credentials for feeds that need one. It's kept beside the
list rather than under it because ticking a feed is supposed to visibly change
it. What you download is exactly what's in it.

You shouldn't lose a zone by accident: the document is mirrored to
`localStorage` as you work and reopened on your next visit, and dropping a saved
`zone.json` onto the page restores the area, the selection and the keys. A
restored document that doesn't parse is left in the pane with the error rather
than discarded — it's usually one character from being valid, and deleting it is
your call.

Feeds with realtime show green tags for the streams they publish — trip updates,
vehicle positions, alerts — and a warning if the realtime side needs its own
credential. Realtime isn't separately selectable, because it has no extent of its
own: GTFS-RT feeds carry no stops, so there's no such thing as realtime "in" an
area independent of the schedule it updates. They reach a zone by hanging off a
static feed, matched through an operator the two share.

The OTP updaters those realtime feeds generate are part of the zone document, as
`router_config`. There's no separate realtime download: the zone file is the
whole configuration, and `bin/k8s/generate` lifts that section straight into the
ConfigMap.

That operator join is loose in places — Coach USA's national record pulls in a
Wisconsin realtime feed for a Bay Area query, for instance — which is another
reason to look at the list rather than take it wholesale.

Notes:

- The basemap comes from the public maps.earth tileserver — the same style the
  frontend uses — so it works with nothing else running. Point `--map-style` at
  a local one to skip the round trip, e.g.
  `--map-style http://localhost:8080/tileserver/style/basic-v2`.
- `--atlas-path` is needed as well as the index because the index stores
  geometry, while the provider name, URL and authorization that go in the zone
  file only exist in DMFR.
- The picker never touches the network for feed data, so it can't discover a
  feed the index is missing. If something you expect isn't there, it's missing
  from the index — check the failures from step 1.
- The zone builder server is a separate crate, so GTFS pipeline builds do not
  pay Actix's compile time. `bin/start-zone-builder-server` builds it after
  verifying its local assets.

## The zone file

What the picker produces is a zone: an area, the feeds serving it, the
credentials to fetch them, and the realtime config for serving them. One file,
`zone.json`, defined in [`src/zone.rs`](src/zone.rs) — Rust is where the schema
lives because Rust is what parses it.

```json
{
  "version": 1,
  "bounds": {
    "min_lon": -122.462, "min_lat": 47.394,
    "max_lon": -122.005, "max_lat": 47.831
  },
  "feeds": [
    {
      "feed_onestop_id": "f-c23-kingcountymetro",
      "provider": "King County Metro",
      "url": "https://metro.kingcounty.gov/GTFS/google_transit.zip",
      "realtime": [
        {
          "feed_onestop_id": "f-c23-kingcountymetro~rt",
          "kinds": ["trip updates", "alerts"],
          "authorization": {
            "type": "header",
            "param_name": "Authorization",
            "info_url": "https://…/register",
            "credential": ""
          }
        }
      ]
    }
  ],
  "router_config": {
    "updaters": [
      {
        "feedId": "headway-f-c23-kingcountymetro",
        "type": "stop-time-updater",
        "frequency": "60s",
        "url": "https://…/trip-updates?key=${HEADWAY_GTFS_API_KEY_F_C23_KINGCOUNTYMETRO_RT}"
      }
    ]
  }
}
```

### Fields

| field | meaning |
|---|---|
| `version` | Schema version, currently `1`. |
| `bounds` | The drawn box: `min_lon`, `min_lat`, `max_lon`, `max_lat`. |
| `feeds[]` | The zone's static GTFS feeds, in the order the picker listed them: best match for the drawn area first. |
| `feeds[].feed_onestop_id` | Transitland ID. The build derives OTP's `feed_id` from it as `headway-<id>`. |
| `feeds[].provider` | Display name, from DMFR. For humans reading the file. |
| `feeds[].url` | Where the archive is fetched from. |
| `feeds[].authorization` | Present only when the feed needs a credential. |
| `feeds[].realtime[]` | GTFS-RT feeds updating this one. Omitted when there are none. |
| `feeds[].realtime[].kinds` | Which streams it publishes: `trip updates`, `vehicle positions`, `alerts`. |
| `authorization.type` | `query_param`, `header` or `replace_url`, from DMFR. |
| `authorization.param_name` | The parameter or header the credential goes in. |
| `authorization.info_url` | Where to request one, when the atlas says. |
| `authorization.credential` | The credential itself — blank until you fill it in. |
| `router_config` | OTP's `router-config.json`: `{"updaters": [...]}`, credentials as `${VAR}`. |

A few things about it worth knowing before the two consumers below make sense:

- **`bounds` is the discovery box, not the routing area.** It's the box you drew,
  recorded so reopening the file restores the area instead of making you find it
  again by eye. The OSM extract the graph is built from comes from the stops in
  the downloaded feeds (`gtfs-bbox`), which is generally larger — a feed only
  clipping your box still enters the graph whole.
- **`feeds` is not derivable from `bounds`.** It's a curated list, and it can
  name a feed the box doesn't touch: the picker keeps a feed you picked when you
  later resize past it, on the grounds that a deliberate choice shouldn't be
  undone by a drag. Re-running the spatial query over `bounds` would produce a
  different, wrong answer — read the list.
- **Nothing derived is stored.** No widths, no areas, no per-feed extents: the
  index has all of that, fresher, and the picker asks it for the feeds a zone
  names (`GET /api/feeds/<ids>`) rather than trusting a snapshot. A zone file
  says which feeds, not where they were the day it was written.
- **Realtime hangs off the static feed it updates.** It has no extent, so this
  nesting is the only way it reaches a zone.
- **`router_config` is OTP's file verbatim**, derived from those realtime
  entries but stored rather than left to be regenerated — it's the half a
  deployment consumes, and the half worth hand-editing (dropping a chatty
  stream, slowing a frequency).
- **`credential` is blank rather than absent when unsupplied**, so the field
  shows up in the file as something to fill in.
- **The credentials are literal, the updaters' are not.** A `${VAR}` in an
  updater is substituted by OTP from its own environment. The two halves have
  different destinations: one is a build input, the other ends up in a
  ConfigMap, so only the first can hold a value. See [`src/api_keys.rs`](src/api_keys.rs)
  for how the variable is named.
- **A filled-in zone file is not safe to commit**, because the keys are in it. A
  zone with its credentials left blank is — that's how the ones in this repo are
  checked in, with the keys in `gtfs-credentials.yaml` instead. The
  `router_config` is safe either way, which is the point of the placeholders.
- **`version` is bumped when a field changes meaning**, so a reader can refuse a
  file it would misinterpret rather than quietly build the wrong zone.

### Use 1: building the graph

A zone's build artifact is essentially one file — `<Area>-<Zone>-<date>.graph.obj.zst`,
the OTP graph — plus the repacked GTFS beside it. The zone file is what the build
needs to produce it:

1. `download-feeds` fetches each `url`, using `authorization` where the feed
   demands it, and repacks each archive with a `feed_info.txt` naming it
   `headway-<feed_onestop_id>`. That id is derived, not stored — which is what
   lets the realtime config in use 2 refer to feeds by name without the two
   files having to agree on anything but the Onestop ID.
2. `gtfs-bbox` reads the stops out of the downloaded feeds to get the zone's real
   extent, and the planet PBF is clipped to it.
3. OTP builds the graph from that clip plus the feeds.

None of this is run by hand normally; `bin/build-transit` drives it through
dagger. See [BUILD.md](../../../BUILD.md).

### Use 2: configuring the OTP nodes

Serving a zone means an OTP process with the graph loaded and a
`router-config.json` telling it where to poll for realtime. That file is the
zone's `router_config`, lifted out as-is — `updaters` keyed by the same
`headway-<onestop-id>` the graph was built with, which is how OTP knows which
static feed a stream updates. Nothing regenerates it at deploy time, so what you
reviewed in the picker is what runs.

In Kubernetes, `bin/k8s/generate` reads that section out of the zone file and
renders it into a ConfigMap per zone
(`k8s/configs/<config>/opentripplanner-<zone>-config.yaml`) as `router-config-json`
alongside `graph-url`; `services/otp/init.sh` writes it out from
`$OTP_ROUTER_CONFIG_JSON` at startup. Realtime credentials are deployed
separately — `bin/k8s-import-transit-tokens`, which reads the `${VAR}` names out
of the same updaters — rather than baked into the ConfigMap.

The important consequence: **realtime is deploy-time configuration, not part of
the artifact.** Adding or re-keying a realtime feed is a ConfigMap change and a
restart, not a graph rebuild. Adding a *static* feed is a rebuild.

### Where it lives, and who reads it

A zone file is `builds/<config>/transit/zones/<zone>.json`, and the file name is
the zone name: `puget-sound.json` produces
`<Area>-puget-sound-<date>-<hash>.graph.obj.zst` and an `otp-graphs` entry keyed
`puget-sound` in `artifacts.json`. That directory is the only place the build
looks — the curated `gtfs_feeds.csv` it replaces is gone. `bin/build-transit`
lints the name as a k8s object name, since it also names the OTP deployment.

| consumer | reads |
|---|---|
| the picker | the only thing that writes a zone; reopens one dropped onto the page |
| dagger / `bin/build-transit` | every `transit/zones/*.json`, one build per file |
| `download-feeds` | the zone file, via `--zone` |
| credentials | the zone's own, with `gtfs-credentials.yaml` filling in any it leaves blank |
| `bin/k8s/generate` | the zone's `router_config`, into the OTP ConfigMap |
| `bin/k8s-import-transit-tokens` | the `${VAR}` names in those updaters, to load the matching Secret |

**Credentials decide whether a zone is committable.** The zone files in this
repo have blank `credential` fields, so they're checked in like any other
config and the keys live in `gtfs-credentials.yaml`, which is gitignored. A zone
you typed keys into is a different thing — keep that one out of git. dagger
mounts every zone file as a secret rather than guessing which kind it has.

## The other binaries

| binary | what it does |
|---|---|
| `write-gtfs-index` | Measures feeds into the GeoPackage index. `--all` or `--feed <id>`. |
| `zone-builder-server` | The map that turns that index into a zone file. |
| `download-feeds` | Fetches and repacks a zone's feeds, for a zone build. |
| `gtfs-bbox` | Computes the bbox of a set of unpacked GTFS directories. |
| `assume-bikes-allowed` | Rewrites a feed to permit bikes, for zones whose data omits it. |

## Development

```
cd ../..
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

This is what `bin/pre-commit rust` runs.
