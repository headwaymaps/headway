# Transit routing

Start here only after the ordinary build in [BUILD.md](./BUILD.md) works.

1. In `builds/Amsterdam/.env`, set `HEADWAY_ENABLE_TRANSIT_ROUTING=1`. Create
   a lowercase zone directory; replace `Amsterdam` below with your build name.

   ```sh
   mkdir -p builds/Amsterdam/transit/amsterdam
   ```
2. Generate the gitignored credentials template and fill in any tokens you have.
   This clones the Transitland Atlas but measures nothing, so it's quick:

   ```sh
   bin/build-gtfs-index --dry-run --write-config-template gtfs-credentials.env
   ```

3. Build the feed-extents index, which says which feeds cover which area. This
   downloads and measures every catalogued feed, so the first run is slow (~30m);
   after that it updates in place, skipping feeds it already measured and
   retrying the ones that failed. Re-run it whenever you add tokens to
   `gtfs-credentials.env`, to measure the feeds those tokens unlock.

   ```sh
   bin/build-gtfs-index
   ```

   Only step 4 needs a local index. The transit-zoner image instead downloads
   the copy published at `gtfs/feed-extents.gpkg` in
   [headway-data](https://github.com/headwaymaps/headway-data).

4. Start transit-zoner, choose the feeds for your area, and save the result as
   `builds/Amsterdam/transit/amsterdam/zone.json`. Use lowercase letters,
   numbers, and dashes for the zone name.

   ```sh
   services/gtfs/transit-zoner/start-dev-server
   ```

5. Generate the zone's credential file. This fails, naming what to add to
   `gtfs-credentials.env`, if a feed your zone uses has no token yet:

   ```sh
   bin/transit-credentials builds/Amsterdam
   ```

   Optionally verify the tokens against the Atlas endpoints before building:

   ```sh
   bin/transit-credentials --verify builds/Amsterdam
   ```

6. Build transit and restart the local stack:

   ```sh
   bin/build-transit builds/Amsterdam
   bin/reset-services builds/Amsterdam
   ```

Update `gtfs-credentials.env` when tokens change, then rerun steps 3 and 5.
Do not edit the generated `transit/<zone>/.env` files directly.
