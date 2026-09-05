# zone-builder-server

The interactive web app for defining transit zones from the GTFS feed-extents
index. It is a separate crate so the GTFS pipeline tools do not depend on
Actix.

Build the local Atlas clone and feed-extents index once:

```sh
bin/zone-builder-build-assets
```

Then start the server:

```sh
bin/zone-builder-server
```

## Tests

The page is one `<script type="module">` inside `assets/`, which the binary
embeds with `include_str!`. `tests/` drives that same file in jsdom - the
harness lifts the script out of the HTML, points its one CDN import at a local
stub of MapLibre, and runs it against a stub of this server's API. So there is
no second copy of the page to keep in step, and the tests exercise what the
binary actually serves.

```sh
yarn install
yarn test
```

They cover what the picker is responsible for: proposing candidates without
choosing any, never silently dropping a deliberate pick when the box moves,
keeping hand-typed credentials across a regenerate, and reopening a zone file
this repository ships. The other half of that last seam - that the build parses
what the picker writes - is `gtfout::zone`'s
`what_the_picker_writes_is_what_the_build_reads`.
