
CITIES = Aachen Aarhus Adelaide Albuquerque Alexandria Amsterdam Antwerpen Arnhem Auckland Augsburg Austin Baghdad Baku Balaton Bamberg Bangkok Barcelona Basel Beijing Beirut Berkeley Berlin Bern Bielefeld Birmingham Bochum Bogota Bombay Bonn Bordeaux Boulder BrandenburgHavel Braunschweig Bremen Bremerhaven Brisbane Bristol Brno Bruegge Bruessel Budapest BuenosAires Cairo Calgary Cambridge CambridgeMa Canberra CapeTown Chemnitz Chicago ClermontFerrand Colmar Copenhagen Cork Corsica Corvallis Cottbus Cracow CraterLake Curitiba Cusco Dallas Darmstadt Davis DenHaag Denver Dessau Dortmund Dresden Dublin Duesseldorf Duisburg Edinburgh Eindhoven Emden Erfurt Erlangen Eugene Flensburg FortCollins Frankfurt FrankfurtOder Freiburg Gdansk Genf Gent Gera Glasgow Gliwice Goerlitz Goeteborg Goettingen Graz Groningen Halifax Halle Hamburg Hamm Hannover Heilbronn Helsinki Hertogenbosch Huntsville Innsbruck Istanbul Jena Jerusalem Johannesburg Kaiserslautern Karlsruhe Kassel Katowice Kaunas Kiel Kiew Koblenz Koeln Konstanz LakeGarda LaPaz LaPlata Lausanne Leeds Leipzig Lima Linz Lisbon Liverpool Ljubljana Lodz London Luebeck Luxemburg Lyon Maastricht Madison Madrid Magdeburg Mainz Malmoe Manchester Mannheim Marseille Melbourne Memphis MexicoCity Miami Minsk Moenchengladbach Montevideo Montpellier Montreal Moscow Muenchen Muenster NewDelhi NewOrleans NewYork Nuernberg Oldenburg Oranienburg Orlando Oslo Osnabrueck Ostrava Ottawa Paderborn Palma PaloAlto Paris Perth Philadelphia PhnomPenh Portland PortlandME Porto PortoAlegre Potsdam Poznan Prag Providence Regensburg Riga RiodeJaneiro Rostock Rotterdam Ruegen Saarbruecken Sacramento Saigon Salzburg SanFrancisco SanJose SanktPetersburg SantaBarbara SantaCruz Santiago Sarajewo Schwerin Seattle Seoul Sheffield Singapore Sofia Stockholm Stockton Strassburg Stuttgart Sucre Sydney Szczecin Tallinn Tehran Tilburg Tokyo Toronto Toulouse Trondheim Tucson Turin UlanBator Ulm Usedom Utrecht Vancouver Victoria WarenMueritz Warsaw WashingtonDC Waterloo Wien Wroclaw Wuerzburg Wuppertal Zagreb Zuerich

.DEFAULT_GOAL := help

help:
	@echo "Try 'make Amsterdam'"
	@echo "Docker must be installed"
	@echo "'make list' for all available metro areas."

list:
	@echo ${CITIES}

%.osm.pbf:
	@echo "Downloading $@ from BBBike.";
	wget -O $@ "https://download.bbbike.org/osm/bbbike/$(basename $(basename $@))/$@" || rm $@
	@echo "\n\nConsider donating to BBBike to help cover hosting! https://extract.bbbike.org/community.html\n\n"

%.pgsql.tgz: %.osm.pbf
	@echo "Extracting $(basename $(basename $@)) data for PostGIS."
	mkdir -p ./.tmp_osm
	rm -rf ./.tmp_osm/*
	cp $(basename $(basename $@)).osm.pbf ./.tmp_osm
	docker run --memory=8G -v ${PWD}/.tmp_osm:/tmp_volume -it --rm osmosis /osmosis/bin/osmosis -v --read-pbf-fast file="/tmp_volume/$(basename $(basename $@)).osm.pbf" --used-node --write-pgsql-dump enableBboxBuilder=yes enableLinestringBuilder=yes directory=/tmp_volume keepInvalidWays=no 
	mkdir -p ./.tmp_osm/$(basename $(basename $@))
	cp ./.tmp_osm/*.txt ./.tmp_osm/$(basename $(basename $@))
	tar -C ./.tmp_osm/$(basename $(basename $@)) -czf $@ nodes.txt relation_members.txt relations.txt users.txt way_nodes.txt ways.txt
	rm -rf ./.tmp_osm/*

%.postgis_deploy_image: %.pgsql.tgz
	@echo "Building PostGIS data deployment job for $(basename $@)"
	rm -f ./postgis_deploy/*.pgsql.tgz
	cp $(basename $@).pgsql.tgz ./postgis_deploy
	docker build ./postgis_deploy --tag headway_postgis_deploy

%.nominatim.tgz: %.osm.pbf
	@echo "Bootstrapping geocoding index for $(basename $(basename $@))."
	mkdir -p ./.tmp_geocoder
	rm -rf ./.tmp_geocoder/*
	cp $(basename $(basename $@)).osm.pbf ./.tmp_geocoder/data.osm.pbf
	docker build ./geocoder/nominatim --tag headway_nominatim
	docker run --memory=8G -v ${PWD}/.tmp_geocoder:/tmp_volume -it --rm \
		-e PBF_PATH=/tmp_volume/data.osm.pbf \
		--name nominatim \
		headway_nominatim
	tar -C ./.tmp_geocoder -czf $@ nominatim
	rm -rf ./.tmp_geocoder/*

%.photon_image: %.nominatim.tgz
	@echo "Importing data into photon and building image for $(basename $@)."
	cp $(basename $@).nominatim.tgz ./geocoder/photon/data.nominatim.tgz
	docker build ./geocoder/photon --tag headway_photon

%.tag_images: %.postgis_deploy_image %.photon_image
	@echo "Tagging images"

$(filter %,$(CITIES)): %: docker %.osm.pbf %.pgsql.tgz %.nominatim.tgz %.tag_images
	@echo "Building $@"

clean:
	rm -rf ./*.pgsql.tgz
	rm -rf ./*.nominatim.tgz
	rm -rf ./.tmp_osm/*
	rm -rf ./.tmp_geocoder/*

clean_all: clean
	rm -rf ./*.osm.pbf

docker:
	@echo "Building Osmosis docker image."
	docker build ./osmosis --tag headway_osmosis