## Supported Build Methods

Headway can be built using a BBBike extract if one exists for a metro area you're interested in, or you can supply your own `.osm.pbf` file to cover larger areas like US states or European countries.

### Building Headway from a supported BBBike extract

1. Pick a metro area from the `Makefile` like "Amsterdam" or "Denver". These values are case-sensitive.
2. (Optional) Set up GTFS feeds for trip planning and multimodal routing capability. This dramatically increases hardware requirements for large metro areas.

   1. Run `make Amsterdam.enumerate_gtfs_feeds`, replacing "Amsterdam" with your metro area of choice.
   2. Examine `data/Amsterdam.gtfs.csv` and manually edit it if necessary to curate GTFS feeds. Some may have errors, and many may be useless for your purposes.

3. Execute `make Amsterdam` using your chosen metro area.
4. Make a `.env` file with configuration. See/copy `.env.example` for defaults. In particular:

   1. `HEADWAY_PUBLIC_URL` (Optional for fully local setup that is accessed exclusively via the base URL `http://127.0.0.1`) Pick a base URL for the domain you wish to serve on, paying attention to scheme (http vs https), domain and port (if not default). This will look like "https://example.com", "http://maps.my.cool.intranet" or "https://maps.example.com:8080". Please omit the trailing slash.
   2. `HEADWAY_AREA`: The name of the area you ran above.

5. Execute `docker-compose up -d` to bring up a headway server on port 8080.
6. (For https and non-default port use only) reverse-proxy traffic to port 8080.

That's it! In the future I'd like to have a kubernetes config to further productionize this project.

### Building Headway from your own OSM extract

Using a custom OSM extract is a bit more complicated, and less regularly tested. Please report issues if you have any, though. Transit trip planning isn't currently supported for arbitrary OSM extracts, contributions are welcome though!

1. Copy your OSM extract into the `data/` directory, as e.g. `data/california.osm.pbf`.
2. Execute `make california.custom` replacing `california` with the name (no extension) of your OSM extract.
3. Make a `.env` file with configuration. See/copy `.env.example` for defaults. In particular:

   1. `HEADWAY_PUBLIC_URL` (Optional for fully local setup that is accessed exclusively via the base URL `http://127.0.0.1`) Pick a base URL for the domain you wish to serve on, paying attention to scheme (http vs https), domain and port (if not default). This will look like "https://example.com", "http://maps.my.cool.intranet" or "https://maps.example.com:8080". Please omit the trailing slash.
   2. `HEADWAY_AREA`: This is the name (no extension) of the OSM extract you used. In this example it would be `california`.
   3. `HEADWAY_FORCE_BBOX`: This is a space-delimited list of lng/lat pairs describing the bounding box of your OSM extract. The format is `west_lng south_lat east_lng north_lat`. The easiest way to get these pairs is probably to go on google maps and estimate the locations for the southwest and northeast points of your extract. You can long-click on a point on the map and it'll show you coordinates. If you generated your OSM extract yourself using Osmium you can just copy the bounding box from the command you used to create it.

4. Execute `docker-compose up -d` to bring up a headway server on port 8080.
5. (For https and non-default port use only) reverse-proxy traffic to port 8080.
