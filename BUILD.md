### Building Headway

1. Pick a metro area from the `Makefile` like "Amsterdam" or "Denver". These values are case-sensitive.
2. Execute `make Amsterdam` using your chosen metro area.
3. Make a `.env` file with configuration. See/copy `.env.example` for defaults. In particular:

    1. `HEADWAY_PUBLIC_URL` (Optional for fully local setup) Pick a base URL for the domain you wish to serve on, paying attention to scheme (http vs https), domain and port (if not default). This will look like "http://example.com" or "http://maps.example.com:8080". Please omit the trailing slash.
    2. `HEADWAY_AREA`: The name of the area you ran above.

4. Execute `docker-compose up -d` to bring up a headway server on port 8080.
5. (For internet-facing installations only) reverse-proxy internet traffic to port 8080.

That's it! In the future I'd like to have a kubernetes config to further productionize this project.
