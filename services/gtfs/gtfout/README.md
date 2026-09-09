# gtfout

`gtfout` downloads, measures, and prepares GTFS feeds for transit builds.

## Feed-extents index

`build-gtfs-index` measures catalogued feeds and stores their coverage in a
GeoPackage. Its defaults are relative to the repo root, so run it from there:

```sh
cargo run --release --package gtfout --bin build-gtfs-index
```

Or point it somewhere else:

```sh
cargo run --release --package gtfout --bin build-gtfs-index -- \
  --atlas-path ./atlas --out ./feed-extents.gpkg
```

Either way the run is incremental: a feed is skipped only once it has been
measured successfully, so re-running retries past failures and is how tokens
added to `gtfs-secrets.json` reach the index. `--dry-run` reports the work without
doing it, and `--write-config-template` writes the `gtfs-secrets.json` skeleton for
the feeds that need a credential.

Every run also writes what the atlas says about each measured feed - provider,
url, authorization and its realtime feeds - into the index, which is where
transit-zoner reads them from.

## Binaries

| Binary | Purpose |
|---|---|
| `build-gtfs-index` | Measure feeds into the GeoPackage index. |
| `transit-credentials` | Write per-zone credential files; `--verify` checks them against Atlas endpoints without writing files. Run through `bin/transit-credentials <build-dir>`. |
| `download-feeds` | Fetch and repack the feeds named by a zone. |
| `gtfs-bbox` | Compute the bounds of unpacked GTFS directories. |
| `assume-bikes-allowed` | Add bike permissions to feeds that omit them. |
| `zone-router-config` | Render a zone's OTP router config with its credentials resolved, or name the feeds whose credentials it needs. Run through `bin/zone-router-config --zone <zone.json>`. |

## Development

```sh
cargo test --package gtfout
```
