# Building Headway

Setting up your own Headway instance should be fairly straightforward if you follow these docs. Feel free to open bugs if things go wrong, or PRs to improve the project though!

## Install Earthly

Headway processes data and builds its Docker images using Earthly. Visit [earthly.dev](https://earthly.dev/) for information about Earthly, and documentation on how to install it.

## Supported Build Methods

Headway can be built using a BBBike extract if one exists for a metro area you're interested in, or you can supply your own `.osm.pbf` file to cover areas that BBBike doesn't cover, or larger areas like US states or European countries.

### Building Headway from a supported BBBike extract

This section pertains to builds from BBBike extracts. Skip this if you know you need to bring your own OpenStreetMap extract.

#### Currently supported cities

Headway currently supports fully automatic builds for the following cities:

Aachen, Aarhus, Adelaide, Albuquerque, Alexandria, Amsterdam, Antwerpen, Arnhem, Auckland, Augsburg, Austin, Baghdad, Baku, Balaton, Bamberg, Bangkok, Barcelona, Basel, Beijing, Beirut, Berkeley, Berlin, Bern, Bielefeld, Birmingham, Bochum, Bogota, Bombay, Bonn, Bordeaux, Boulder, BrandenburgHavel, Braunschweig, Bremen, Bremerhaven, Brisbane, Bristol, Brno, Bruegge, Bruessel, Budapest, BuenosAires, Cairo, Calgary, Cambridge, CambridgeMa, Canberra, CapeTown, Chemnitz, Chicago, ClermontFerrand, Colmar, Copenhagen, Cork, Corsica, Corvallis, Cottbus, Cracow, CraterLake, Curitiba, Cusco, Dallas, Darmstadt, Davis, DenHaag, Denver, Dessau, Dortmund, Dresden, Dublin, Duesseldorf, Duisburg, Edinburgh, Eindhoven, Emden, Erfurt, Erlangen, Eugene, Flensburg, FortCollins, Frankfurt, FrankfurtOder, Freiburg, Gdansk, Genf, Gent, Gera, Glasgow, Gliwice, Goerlitz, Goeteborg, Goettingen, Graz, Groningen, Halifax, Halle, Hamburg, Hamm, Hannover, Heilbronn, Helsinki, Hertogenbosch, Huntsville, Innsbruck, Istanbul, Jena, Jerusalem, Johannesburg, Kaiserslautern, Karlsruhe, Kassel, Katowice, Kaunas, Kiel, Kiew, Koblenz, Koeln, Konstanz, LakeGarda, LaPaz, LaPlata, Lausanne, Leeds, Leipzig, Lima, Linz, Lisbon, Liverpool, Ljubljana, Lodz, London, Luebeck, Luxemburg, Lyon, Maastricht, Madison, Madrid, Magdeburg, Mainz, Malmoe, Manchester, Mannheim, Marseille, Melbourne, Memphis, MexicoCity, Miami, Minsk, Moenchengladbach, Montevideo, Montpellier, Montreal, Moscow, Muenchen, Muenster, NewDelhi, NewOrleans, NewYork, Nuernberg, Oldenburg, Oranienburg, Orlando, Oslo, Osnabrueck, Ostrava, Ottawa, Paderborn, Palma, PaloAlto, Paris, Perth, Philadelphia, PhnomPenh, Portland, PortlandME, Porto, PortoAlegre, Potsdam, Poznan, Prag, Providence, Regensburg, Riga, RiodeJaneiro, Rostock, Rotterdam, Ruegen, Saarbruecken, Sacramento, Saigon, Salzburg, SanFrancisco, SanJose, SanktPetersburg, SantaBarbara, SantaCruz, Santiago, Sarajewo, Schwerin, Seattle, Seoul, Sheffield, Singapore, Sofia, Stockholm, Stockton, Strassburg, Stuttgart, Sucre, Sydney, Szczecin, Tallinn, Tehran, Tilburg, Tokyo, Toronto, Toulouse, Trondheim, Tucson, Turin, UlanBator, Ulm, Usedom, Utrecht, Vancouver, Victoria, WarenMueritz, Warsaw, WashingtonDC, Waterloo, Wien, Wroclaw, Wuerzburg, Wuppertal, Zagreb, Zuerich

#### Build procedure.

1. Pick a metro area from the list above, like "Amsterdam" or "Denver". These values are case-sensitive.
2. (Optional) Set up GTFS feeds for trip planning and multimodal routing capability. This dramatically increases hardware requirements for large metro areas.

   1. Run `earthly +gtfs-enumerate --area="Amsterdam"`, replacing "Amsterdam" with your metro area of choice.
   2. Examine `data/Amsterdam.gtfs.csv` and manually edit it if necessary to curate GTFS feeds. Some may have errors, and many may be useless for your purposes.

3. Execute `earthly +build --area="Amsterdam"` using your chosen metro area.
4. Make a `.env` file with configuration. See/copy `.env.example` for defaults. In particular:

   1. `HEADWAY_PUBLIC_URL` (Optional for fully local setup that is accessed exclusively via the base URL `http://127.0.0.1`) Pick a base URL for the domain you wish to serve on, paying attention to scheme (http vs https), domain and port (if not default). This will look like "https://example.com", "http://maps.my.cool.intranet" or "https://maps.example.com:8080". Please omit the trailing slash.
   2. `HEADWAY_AREA`: The name of the area you ran above. In our case, "Amsterdam".

5. Execute `docker-compose up -d` to bring up a headway server on port 8080.
6. (For https and non-default port use only) reverse-proxy traffic to port 8080.

That's it! In the future I'd like to have a kubernetes config to further productionize this project.

### Building Headway from your own OSM extract

Using a custom OSM extract is a bit more complicated, and less regularly tested. Please report issues if you have any. Transit trip planning isn't currently supported for arbitrary OSM extracts, contributions are welcome though!

1. Copy your OSM extract into Headway's top-level directory (same directory as this file), as e.g. `./california.osm.pbf`. It is important to name it something different than the cities listed above. For example, if I was building a custom extract of Amsterdam to avoid conflicts I would name it `AmsterdamCustom`.
2. Execute `earthly +build --area="california" --countries="US"` replacing `california` with the name (no extension) of your OSM extract, and `US` with a comma-separated list of the countries that the extract covers. This specifies which data to download from Who's On First. Accidentally including a country won't harm anything but it will cause needless downloads. You may wish to simply put `*` which will download data for the whole planet (potentially tens of gigabytes).
3. Make a `.env` file with configuration. See/copy `.env.example` for defaults. In particular:

   1. `HEADWAY_PUBLIC_URL` (Optional for fully local setup that is accessed exclusively via the base URL `http://127.0.0.1`) Pick a base URL for the domain you wish to serve on, paying attention to scheme (http vs https), domain and port (if not default). This will look like "https://example.com", "http://maps.my.cool.intranet" or "https://maps.example.com:8080". Please omit the trailing slash.
   2. `HEADWAY_AREA`: This is the name (no extension) of the OSM extract you used. In this example it would be `california`.
   3. `HEADWAY_FORCE_BBOX`: This is a space-delimited list of lng/lat pairs describing the bounding box of your OSM extract. The format is `west_lng south_lat east_lng north_lat`. The easiest way to get these pairs is probably to go on google maps and estimate the locations for the southwest and northeast points of your extract. You can long-click on a point on the map and it'll show you coordinates. If you generated your OSM extract yourself using Osmium you can just copy the bounding box from the command you used to create it.

4. Execute `docker-compose up -d` to bring up a headway server on port 8080.
5. (For https and non-default port use only) reverse-proxy traffic to port 8080.

## Docker-compose restarts

Because Headway's docker-compose configuration uses init containers to populate a docker volume containing internal data, rebuilding the data for a metro area won't update existing containers. It's necessary to run `docker-compose down --volume` to re-initialize the data in the init containers.

This is necessary whenever you rebuild the data for a metro area, or change which area you're serving data for in the `.env` file.
