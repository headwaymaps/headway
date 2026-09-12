# Transit routing

Start here only after the ordinary build in [BUILD.md](./BUILD.md) works.

1. Create a lowercase zone directory; replace `Amsterdam` below with your build
   name. A build has transit routing when it has a zone, so there is no flag to
   set.

   ```sh
   mkdir -p builds/Amsterdam/transit/amsterdam
   ```

2. Generate the gitignored credentials template and fill in any tokens you have.
   This clones the Transitland Atlas but measures nothing, so it's quick:

   ```sh
   bin/build-gtfs-index --dry-run --write-config-template gtfs-secrets.json
   ```

   Credentials live in `gtfs-secrets.json`.

3. Build the feed-extents index, which says which feeds cover which area.

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

5. Give each zone its own slice of the credentials. This writes
   `transit/<zone>/gtfs-secrets.json`.

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

Update `gtfs-secrets.json` when tokens change, then rerun steps 3 and 5.
