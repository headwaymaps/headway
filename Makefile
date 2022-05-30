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
DATA_DIR := "${PWD}/data"
DOCKER_MEMORY := "12G"
JAVA_TOOL_OPTIONS := "-Xmx12G"

help:
	@echo "Try 'make Amsterdam'"
	@echo "Docker must be installed"
	@echo "'make list' for all available metro areas."

list:
	@echo ${CITIES}

%.osm.pbf:
	@echo "Downloading $@ from BBBike.";
	@echo "\n\nConsider donating to BBBike to help cover hosting! https://extract.bbbike.org/community.html\n\n"
	wget -U headway/1.0 -O $@ "https://download.bbbike.org/osm/bbbike/$(notdir $(basename $(basename $@)))/$(notdir $@)" || rm $@

%.bbox:
	@echo "Extracting bounding box for $(notdir $(basename $@))"
	grep "$(notdir $(basename $@)):" gtfs/bboxes.csv > $@
	perl -i.bak -pe 's/$(notdir $(basename $@))://' $@

%.mbtiles: %.osm.pbf
	@echo "Building MBTiles $(notdir $(basename $@))"
	mkdir -p data/sources
	cd data/sources && bash -c "find -name '*.zip' || (wget https://f000.backblazeb2.com/file/headway/sources.tar && tar xvf sources.tar)"
	docker run --memory=$(DOCKER_MEMORY) --rm -e JAVA_TOOL_OPTIONS="$(JAVA_TOOL_OPTIONS)" \
		-v "${DATA_DIR}:/data" \
		ghcr.io/onthegomap/planetiler:latest \
		--osm-path=/data/$(notdir $(basename $(basename $@))).osm.pbf \
		--mbtiles=/data/$(notdir $(basename $(basename $@))).mbtiles \
		--download \
		--force

%.nominatim.sql: %.osm.pbf
	@echo "Bootstrapping geocoding index for $(basename $(basename $@))."
	docker run --memory=$(DOCKER_MEMORY) -it --rm \
		-v "${DATA_DIR}":/data \
		-v "${DATA_DIR}/nominatim_pg/":/var/lib/postgresql/12/main \
		-v "${DATA_DIR}/nominatim_flatnode/":/nominatim/flatnode \
		-e PBF_PATH=/data/$(notdir $(basename $(basename $@))).osm.pbf \
		mediagis/nominatim:4.0 \
		bash -c 'useradd -m nominatim && /app/config.sh && /app/init.sh && touch /var/lib/postgresql/12/main/import-finished && service postgresql start && (sudo -u nominatim pg_dump nominatim > /data/$(notdir $(basename $(basename $@))).nominatim.sql)'

photon_image:
	@echo "Building photon image"
	docker build ./geocoder/photon --tag headway_photon

%.photon: %.nominatim.sql photon_image
	@echo "Importing data into photon for $(basename $@)."
	mkdir -p $@
	docker run -it --rm \
		-v "${DATA_DIR}":/data \
		-v "$@":/photon_data \
		-e HEADWAY_NOMINATIM_FILE=/data/$(notdir $(basename $(basename $@))).nominatim.sql \
		-w /photon_data \
		--user 0 \
		headway_photon \
		/photon/import_from_dump.sh

%.graph.tgz: %.osm.pbf
	@echo "Pre-generating graphhopper graph for $(basename $(basename $@))."
	docker build ./graphhopper --tag headway_graphhopper_build_image
	mkdir -p ./.tmp_graphhopper
	rm -rf ./.tmp_graphhopper/*
	cp $(basename $(basename $@)).osm.pbf ./.tmp_graphhopper/data.osm.pbf
	-docker volume rm -f headway_graphhopper_build || echo "Volume does not exist!"
	docker volume create headway_graphhopper_build
	docker run -d --rm --name headway_graphhopper_ephemeral_busybox_build \
		-v headway_graphhopper_build:/headway_graphhopper_build \
		alpine:3 \
		sleep 1000
	docker ps -aqf "name=headway_graphhopper_ephemeral_busybox_build" > .graphhopper_build_cid
	bash -c 'docker cp $(basename $(basename $@)).osm.pbf $$(<.graphhopper_build_cid):/headway_graphhopper_build/data.osm.pbf'
	-bash -c 'docker kill $$(<.graphhopper_build_cid) || echo "container is not running"'
	docker run --memory=$(DOCKER_MEMORY) -it --rm \
		-v headway_graphhopper_build:/graph_volume \
		headway_graphhopper_build_image \
		-Ddw.graphhopper.datareader.file=/graph_volume/data.osm.pbf \
		-jar \
		/graphhopper/graphhopper-web-5.3.jar \
		import \
		config.yaml
	-docker ps -aqf "name=headway_graphhopper_ephemeral_busybox_build" > .graphhopper_build_cid
	-bash -c 'docker kill $$(<.graphhopper_build_cid) || echo "container is not running"'
	docker run --rm \
		-v headway_graphhopper_build:/headway_graphhopper_build \
		alpine:3 \
		/bin/sh -c 'rm -f /headway_graphhopper_build/graph.tgz && cd /headway_graphhopper_build && tar -czf graph.tgz *'
	docker run -d --rm --name headway_graphhopper_ephemeral_busybox_build \
		-v headway_graphhopper_build:/headway_graphhopper_build \
		alpine:3 \
		sleep 1000
	docker ps -aqf "name=headway_graphhopper_ephemeral_busybox_build" > .graphhopper_build_cid
	bash -c 'docker cp $$(<.graphhopper_build_cid):/headway_graphhopper_build/graph.tgz $@'
	-bash -c 'docker kill $$(<.graphhopper_build_cid) || echo "container is not running"'
	rm -rf ./.tmp_graphhopper/*

nginx_image:
	docker build ./web --tag headway_nginx

tag_images: nginx_image photon_image graphhopper_image
	@echo "Tagging images"

%.graphhopper_volume: %.graph.tgz graphhopper_image
	@echo "Create volume, then delete, then create, to force failures if the volume is in use."
	-docker volume create headway_graphhopper_vol
	docker volume rm -f headway_graphhopper_vol
	docker volume create headway_graphhopper_vol

	docker run -d --rm --name headway_graphhopper_ephemeral_busybox_tag \
		-v headway_graphhopper_vol:/headway_graphhopper \
		alpine:3 \
		sleep 1000

	-docker ps -aqf "name=headway_graphhopper_ephemeral_busybox_tag" > .graphhopper_cid
	bash -c 'docker cp $(basename $@).graph.tgz $$(<.graphhopper_cid):/headway_graphhopper/graph.tgz'
	-bash -c 'docker kill $$(<.graphhopper_cid) || echo "container is not running"'

	docker run --rm \
		-v headway_graphhopper_vol:/headway_graphhopper \
		alpine:3 \
		/bin/sh -c 'cd /headway_graphhopper && tar -xvf graph.tgz && rm graph.tgz'

%.tag_volumes: %.graphhopper_volume
	@echo "Tagged volumes"

$(filter %,$(CITIES)): %: ${DATA_DIR}/%.osm.pbf ${DATA_DIR}/%.nominatim.sql ${DATA_DIR}/%.photon ${DATA_DIR}/%.mbtiles ${DATA_DIR}/%.graph.tgz tag_images %.tag_volumes
	@echo "Building $@"

clean:
	rm -rf ${DATA_DIR}/*.mbtiles
	rm -rf ${DATA_DIR}/*.nominatim.sql
	rm -rf ./.tmp_graphhopper

%.up: % ${DATA_DIR}/%.osm.pbf ${DATA_DIR}/%.nominatim.sql ${DATA_DIR}/%.photon ${DATA_DIR}/%.mbtiles ${DATA_DIR}/%.graph.tgz tag_images %.tag_volumes
	docker-compose kill || echo "Containers not up"
	docker-compose down || echo "Containers dont exist"
	docker-compose up -d

# Don't clean base URL because that's a user config option.
clean_all: clean
	rm -rf ${DATA_DIR}/*.osm.pbf
	rm -rf ${DATA_DIR}/*.bbox
	rm -rf ${DATA_DIR}/*.bbox.bak
	rm -rf ${DATA_DIR}/*.photon
	rm -rf ${DATA_DIR}/sources
	rm -rf ${DATA_DIR}/nominatim_flatnode
	rm -rf ${DATA_DIR}/nominatim_pg

graphhopper_image:
	docker build ./graphhopper --tag headway_graphhopper
