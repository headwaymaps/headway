# Copyright (C) 2022 Ellen Poe

# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.

# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see https://www.gnu.org/licenses/.

CITIES = Aachen Aarhus Adelaide Albuquerque Alexandria Amsterdam Antwerpen Arnhem Auckland Augsburg Austin Baghdad \
				Baku Balaton Bamberg Bangkok Barcelona Basel Beijing Beirut Berkeley Berlin Bern Bielefeld Birmingham Bochum \
				Bogota Bombay Bonn Bordeaux Boulder BrandenburgHavel Braunschweig Bremen Bremerhaven Brisbane Bristol Brno \
				Bruegge Bruessel Budapest BuenosAires Cairo Calgary Cambridge CambridgeMa Canberra CapeTown Chemnitz Chicago \
				ClermontFerrand Colmar Copenhagen Cork Corsica Corvallis Cottbus Cracow CraterLake Curitiba Cusco Dallas \
				Darmstadt Davis DenHaag Denver Dessau Dortmund Dresden Dublin Duesseldorf Duisburg Edinburgh Eindhoven Emden \
				Erfurt Erlangen Eugene Flensburg FortCollins Frankfurt FrankfurtOder Freiburg Gdansk Genf Gent Gera Glasgow \
				Gliwice Goerlitz Goeteborg Goettingen Graz Groningen Halifax Halle Hamburg Hamm Hannover Heilbronn Helsinki \
				Hertogenbosch Huntsville Innsbruck Istanbul Jena Jerusalem Johannesburg Kaiserslautern Karlsruhe Kassel \
				Katowice Kaunas Kiel Kiew Koblenz Koeln Konstanz LakeGarda LaPaz LaPlata Lausanne Leeds Leipzig Lima Linz \
				Lisbon Liverpool Ljubljana Lodz London Luebeck Luxemburg Lyon Maastricht Madison Madrid Magdeburg Mainz \
				Malmoe Manchester Mannheim Marseille Melbourne Memphis MexicoCity Miami Minsk Moenchengladbach Montevideo \
				Montpellier Montreal Moscow Muenchen Muenster NewDelhi NewOrleans NewYork Nuernberg Oldenburg Oranienburg \
				Orlando Oslo Osnabrueck Ostrava Ottawa Paderborn Palma PaloAlto Paris Perth Philadelphia PhnomPenh Portland \
				PortlandME Porto PortoAlegre Potsdam Poznan Prag Providence Regensburg Riga RiodeJaneiro Rostock Rotterdam \
				Ruegen Saarbruecken Sacramento Saigon Salzburg SanFrancisco SanJose SanktPetersburg SantaBarbara SantaCruz \
				Santiago Sarajewo Schwerin Seattle Seoul Sheffield Singapore Sofia Stockholm Stockton Strassburg Stuttgart \
				Sucre Sydney Szczecin Tallinn Tehran Tilburg Tokyo Toronto Toulouse Trondheim Tucson Turin UlanBator Ulm \
				Usedom Utrecht Vancouver Victoria WarenMueritz Warsaw WashingtonDC Waterloo Wien Wroclaw Wuerzburg Wuppertal \
				Zagreb Zuerich

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

%.mbtiles: %.osm.pbf
	@echo "Building MBTiles $(basename $@)"
	mkdir -p ./.tmp_mbtiles
	cp $(basename $@).osm.pbf ./.tmp_mbtiles/data.osm.pbf
	rm -f ./.tmp_mbtiles/output.mbtiles
	docker run --memory=6G --rm -e JAVA_TOOL_OPTIONS="-Xmx8g" -v "${PWD}/.tmp_mbtiles":/data ghcr.io/onthegomap/planetiler:latest --osm-path=/data/data.osm.pbf --download
	mv ./.tmp_mbtiles/output.mbtiles $(basename $@).mbtiles

%.nominatim.tgz: %.osm.pbf
	@echo "Bootstrapping geocoding index for $(basename $(basename $@))."
	mkdir -p ./.tmp_geocoder
	rm -rf ./.tmp_geocoder/*
	cp $(basename $(basename $@)).osm.pbf ./.tmp_geocoder/data.osm.pbf
	docker volume rm headway_geocoder_build || echo "Volume does not exist!"
	docker volume create headway_geocoder_build
	docker build ./geocoder/nominatim --tag headway_nominatim
	docker run --memory=6G -it --rm \
		-v headway_geocoder_build:/tmp_volume \
		-v "${PWD}/.tmp_geocoder":/data_volume \
		-e PBF_PATH=/data_volume/data.osm.pbf \
		--name nominatim \
		headway_nominatim
	docker ps -aqf "name=headway_geocoder_ephemeral_busybox" > .nominatim_cid
	bash -c 'docker kill $$(<.nominatim_cid) || echo "container is not running"'
	bash -c 'docker rm $$(<.nominatim_cid) || echo "container does not exist"'
	docker run -d --name headway_geocoder_ephemeral_busybox -v headway_geocoder_build:/headway_geocoder_build busybox sleep 1000
	docker ps -aqf "name=headway_geocoder_ephemeral_busybox" > .nominatim_cid
	bash -c 'docker cp $$(<.nominatim_cid):/headway_geocoder_build/nominatim ./.tmp_geocoder/nominatim'
	bash -c 'docker kill $$(<.nominatim_cid) || echo "container is not running"'
	bash -c 'docker rm $$(<.nominatim_cid) || echo "container does not exist"'
	tar -C ./.tmp_geocoder -czf $@ nominatim
	rm -rf ./.tmp_geocoder/*

%.photon_image: %.nominatim.tgz
	@echo "Importing data into photon and building image for $(basename $@)."
	cp $(basename $@).nominatim.tgz ./geocoder/photon/data.nominatim.tgz
	docker build ./geocoder/photon --tag headway_photon

%.tileserver_image: %.mbtiles
	@echo "Building tileserver image for $(basename $@)."
	cp $(basename $@).mbtiles ./tileserver/tiles.mbtiles
	docker build ./tileserver --tag headway_tileserver

%.valhalla.tar: %.osm.pbf
	@echo "Building valhalla tiles for $(basename $(basename $@))."
	mkdir -p ./.tmp_valhalla
	rm -rf ./.tmp_valhalla/*
	cp $(basename $(basename $@)).osm.pbf ./.tmp_valhalla/data.osm.pbf
	docker volume rm headway_valhalla_build || echo "Volume does not exist!"
	docker volume create headway_valhalla_build
	docker build ./valhalla/build --tag headway_valhalla_build
	docker run --rm --memory=6G \
		-v headway_valhalla_build:/tmp_vol \
		-v ${PWD}/.tmp_valhalla:/data_vol \
		headway_valhalla_build
	docker ps -aqf "name=headway_valhalla_ephemeral_busybox" > .valhalla_cid
	bash -c 'docker kill $$(<.valhalla_cid) || echo "container is not running"'
	bash -c 'docker rm $$(<.valhalla_cid) || echo "container does not exist"'
	docker run -d --name headway_valhalla_ephemeral_busybox -v headway_valhalla_build:/headway_valhalla_build busybox sleep 1000
	docker ps -aqf "name=headway_valhalla_ephemeral_busybox" > .valhalla_cid
	bash -c 'docker cp $$(<.valhalla_cid):/headway_valhalla_build/valhalla_tiles.tar $@'
	bash -c 'docker kill $$(<.valhalla_cid) || echo "container is not running"'
	bash -c 'docker rm $$(<.valhalla_cid) || echo "container does not exist"'

%.valhalla_image: %.valhalla.tar
	@echo "Building valhalla image for $(basename $@)."
	cp $(basename $@).valhalla.tar ./valhalla/run/tiles.tar
	docker build ./valhalla/run --tag headway_valhalla_run

%.tag_images: %.tileserver_image %.photon_image %.valhalla_image
	@echo "Tagging images"

$(filter %,$(CITIES)): %: %.osm.pbf %.nominatim.tgz %.mbtiles %.tag_images
	@echo "Building $@"

clean:
	rm -rf ./*.valhalla.tar
	rm -rf ./*.nominatim.tgz
	rm -rf ./*.mbtiles
	rm -rf ./.tmp_mbtiles/tmp
	rm -rf ./.tmp_mbtiles/data.osm.pbf
	rm -rf ./.tmp_valhalla/*
	rm -rf ./.tmp_geocoder/*

clean_all: clean
	rm -rf ./*.osm.pbf
	rm -rf ./.tmp_mbtiles/*