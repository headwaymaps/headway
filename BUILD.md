### Building Headway

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
