# Building Headway

Setting up your own Headway instance should be fairly straightforward if you follow these docs. Feel free to open bugs if things go wrong, or submit PRs to improve the project!

There is a script contributed by Santiago Crespo that will automatically deploy Headway as a systemd service on Debian, but it has not been widely tested yet. See [contrib/DEBIAN_BUILD.md](./contrib/DEBIAN_BUILD.md) for details.

Prerequisites: [Install Dagger.](#install-dagger)

[Option 1: Building from a pre-configured city](#building-headway-from-a-supported-bbbike-extract)

[Option 2: Building from your own OSM extract](#building-headway-from-your-own-osm-extract)

[Option 3: Building Headway for the whole planet](#full-planet-considerations)

## Install Dagger

Headway processes data and builds its containers for hosting using Dagger. Dagger is a build system for orchestrating containerized workflows.

Instructions for installing Dagger can be found here: https://docs.dagger.io/install

Dagger is open source and free to use locally without requiring any accounts or cloud services.

## Supported Build Methods

Headway can be built using a BBBike extract if one exists for a metro area you're interested in, or you can supply your own `.osm.pbf` file to cover areas that BBBike doesn't cover, or larger areas like US states or European countries.

### Building Headway from a supported BBBike extract

This section pertains to builds from BBBike extracts. Skip this if you know you need to bring your own OpenStreetMap extract.

#### Currently supported cities

Headway currently supports fully automatic builds for the following cities:

<details>
  <summary>Supported cities</summary>
   Aachen, Aarhus, Adelaide, Albuquerque, Alexandria, Amsterdam, Antwerpen, Arnhem, Auckland, Augsburg, Austin, Baghdad, Baku, Balaton, Bamberg, Bangkok, Barcelona, Basel, Beijing, Beirut, Berkeley, Berlin, Bern, Bielefeld, Birmingham, Bochum, Bogota, Bombay, Bonn, Bordeaux, Boulder, BrandenburgHavel, Braunschweig, Bremen, Bremerhaven, Brisbane, Bristol, Brno, Bruegge, Bruessel, Budapest, BuenosAires, Cairo, Calgary, Cambridge, CambridgeMa, Canberra, CapeTown, Chemnitz, Chicago, ClermontFerrand, Colmar, Copenhagen, Cork, Corsica, Corvallis, Cottbus, Cracow, CraterLake, Curitiba, Cusco, Dallas, Darmstadt, Davis, DenHaag, Denver, Dessau, Dortmund, Dresden, Dublin, Duesseldorf, Duisburg, Edinburgh, Eindhoven, Emden, Erfurt, Erlangen, Eugene, Flensburg, FortCollins, Frankfurt, FrankfurtOder, Freiburg, Gdansk, Genf, Gent, Gera, Glasgow, Gliwice, Goerlitz, Goeteborg, Goettingen, Graz, Groningen, Halifax, Halle, Hamburg, Hamm, Hannover, Heilbronn, Helsinki, Hertogenbosch, Huntsville, Innsbruck, Istanbul, Jena, Jerusalem, Johannesburg, Kaiserslautern, Karlsruhe, Kassel, Katowice, Kaunas, Kiel, Kiew, Koblenz, Koeln, Konstanz, LakeGarda, LaPaz, LaPlata, Lausanne, Leeds, Leipzig, Lima, Linz, Lisbon, Liverpool, Ljubljana, Lodz, London, Luebeck, Luxemburg, Lyon, Maastricht, Madison, Madrid, Magdeburg, Mainz, Malmoe, Manchester, Mannheim, Marseille, Melbourne, Memphis, MexicoCity, Miami, Minsk, Moenchengladbach, Montevideo, Montpellier, Montreal, Moscow, Muenchen, Muenster, NewDelhi, NewOrleans, NewYork, Nuernberg, Oldenburg, Oranienburg, Orlando, Oslo, Osnabrueck, Ostrava, Ottawa, Paderborn, Palma, PaloAlto, Paris, Perth, Philadelphia, PhnomPenh, Portland, PortlandME, Porto, PortoAlegre, Potsdam, Poznan, Prag, Providence, Regensburg, Riga, RiodeJaneiro, Rostock, Rotterdam, Ruegen, Saarbruecken, Sacramento, Saigon, Salzburg, SanFrancisco, SanJose, SanktPetersburg, SantaBarbara, SantaCruz, Santiago, Sarajewo, Schwerin, Seattle, Seoul, Sheffield, Singapore, Sofia, Stockholm, Stockton, Strassburg, Stuttgart, Sucre, Sydney, Szczecin, Tallinn, Tehran, Tilburg, Tokyo, Toronto, Toulouse, Trondheim, Tucson, Turin, UlanBator, Ulm, Usedom, Utrecht, Vancouver, Victoria, WarenMueritz, Warsaw, WashingtonDC, Waterloo, Wien, Wroclaw, Wuerzburg, Wuppertal, Zagreb, Zuerich
</details>

#### Build procedure.

This approach will download all the mapping data you need automatically, but only works for the pre-defined metro areas above.

1. Pick a metro area from the list above, like "Amsterdam" or "Denver". These values are case-sensitive. In all the examples, replace "Amsterdam" with your metro area of choice.
2. Configuration is managed per build directory in `builds/<Area>`. Copy a template build directory: `cp -r builds/Bogota builds/Amsterdam`, review and edit `builds/Amsterdam/.env`. Bogota is configured for transit routing, so unless you're setting that up too (step 4), delete the copied `builds/Amsterdam/transit` directory and unset `HEADWAY_ENABLE_TRANSIT_ROUTING`.
3. Execute `bin/build builds/Amsterdam` to build data artifacts
4. (Optional) Set up transit routing. Note: This increases hosting requirements for large metro areas - you'll want at least 4GB RAM extra for a medium sized city's transit service.
   1. Find nearby transit schedules by running `bin/export-nearby-transit-feeds builds/Amsterdam`
   2. Examine `builds/Amsterdam/transit/gtfs-feeds/amsterdam.gtfs_feeds.csv` and manually edit it if necessary to curate GTFS feeds. Some may have errors, and many may be useless for your purposes.
   3. Build transit routing with `bin/build-transit builds/Amsterdam`
5. Run `bin/start-services builds/Amsterdam`. This will bring up the Headway stack with a web frontend on port 8080.
  1. (Optional for https and non-default port use only) reverse-proxy traffic to port 8080.

That's it!

There are some experimental kubernetes configs in k8s/configs, but they are pretty specific to my own needs at this point.

### Building Headway from your own OSM extract

To build Headway for a custom area, you just need to provide your own OSM extract (.osm.pbf).

The process is largely the same as above. After downloading your OSM extract, move it to the project root (in the same directory as this BUILD.md), and wherever you see `with-area Amsterdam` in the build scripts, change it to `with-area YourArea --local-pbf ./your-area.osm.pbf`.

## Docker-compose restarts

Rebuilding the data for a metro area won't update existing containers.

```
# delete all existing docker data volumes and containers
bin/stop-and-remove-services builds/Amsterdam
# start the services again, which will pull in fresh data
bin/start-services builds/Amsterdam

# or both in a single command
bin/reset-services builds/Amsterdam
```

This is necessary whenever you rebuild the data for a metro area, or change which area you're serving data for in the `builds/<area>/.env` file.

## Kubernetes data volumes

Every service that needs map data mounts a PersistentVolumeClaim whose name encodes the version of the data it holds — `HEADWAY_AREA_TAG` (which contains the planet version) plus `HEADWAY_DATA_TAG`, and for transit, the graph's build date as well. Since data artifacts are immutable once downloaded, this means:

- Restarting a deployment without changing any data reuses the existing volume. The init container sees the artifact is already there and exits immediately, so `kubectl rollout restart` takes seconds instead of re-downloading hundreds of gigabytes.
- Bumping the planet version changes the claim names, so the new pods provision empty volumes and download into them while the old pods keep serving.

### Deploying a new planet version

```sh
bin/update-planet-version                        # bumps builds/planet/.env
bin/build builds/planet
bin/publish-data builds/planet --host <asset-host>
bin/k8s-generate builds/planet k8s/configs/planet
kubectl apply -f k8s/configs/planet
```

### Deploying a transit rebuild

Transit graphs are rebuilt more often than the planet, so the area/data tags alone don't distinguish one graph from the next. The claim name includes the graph's build date as well, so each graph gets its own volume rather than an OTP pod reusing one that already holds an older graph:

```sh
bin/build-transit builds/planet
bin/publish-data builds/planet --host <asset-host>
bin/k8s-generate builds/planet k8s/configs/planet
kubectl apply -f k8s/configs/planet
```

### Reclaiming old volumes

Volumes accumulate: each planet bump and each transit rebuild leaves the previous one behind. That's deliberate — it's what makes a rollback instant. Once you're happy with a rollout, collect them:

```sh
bin/deploy/show-pvcs        # what exists, and whether anything still uses it
bin/deploy/gc-pvcs          # dry run
bin/deploy/gc-pvcs --apply  # delete
```

`gc-pvcs` deletes any headway-labeled PVC that no Deployment references and no running pod mounts, so it can't remove a volume that's actually in use. If your PVs use the `Retain` reclaim policy, it warns that the released PVs still need deleting before the disk is actually freed.

## Full-planet considerations

See [FULL_PLANET.md](./FULL_PLANET.md).
