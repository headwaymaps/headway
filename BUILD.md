### Building Headway

If you want to host a headway instance for the open internet, you need to start with step 1, but for serving on localhost at port 8080 you can start at step 2.

1. (For internet-facing installations only) Pick a base URL for the domain you wish to serve on, paying attention to scheme (http vs https), domain and port (if not default). This will look like "http://example.com" or "http://maps.example.com:8080", and then execute the command `echo "http://example.com" > .base_url` using your chosen base URL. Please omit the trailing slash. In the future it would be nice to be able to tolerate a trailing slash but I doubt the build process can do so for the time being.
2. Pick a metro area from the `Makefile` like "Amsterdam" or "Denver". These values are case-sensitive.
3. Execute `make Amsterdam` using your chosen metro area.
4. Execute `docker-compose up -d` to bring up a headway server on port 8080.
5. (For internet-facing installations only) reverse-proxy internet traffic to port 8080.

That's it! In the future I'd like to have a kubernetes config to further productionize this project.