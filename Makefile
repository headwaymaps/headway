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
DATA_DIR := ${PWD}/data
DOCKER_MEMORY := "12G"
JAVA_TOOL_OPTIONS := "-Xmx12G"

help:
	@echo "Try 'make Amsterdam'"
	@echo "Docker must be installed"
	@echo "'make list' for all available metro areas."

list:
	@echo ${CITIES}

%.osm.pbf:
	@mkdir -p ${DATA_DIR}
	@echo "Downloading $@ from BBBike.";
	@echo "\n\nConsider donating to BBBike to help cover hosting! https://extract.bbbike.org/community.html\n\n"
	wget -U headway/1.0 -O $@ "https://download.bbbike.org/osm/bbbike/$(notdir $(basename $(basename $@)))/$(notdir $@)" || rm $@

%.bbox:
	@echo "Extracting bounding box for $(notdir $(basename $@))"
	grep "$(notdir $(basename $@)):" gtfs/bboxes.csv > ${DATA_DIR}/bbox.txt
	perl -i.bak -pe 's/$(notdir $(basename $@))://' ${DATA_DIR}/bbox.txt

%.mbtiles: %.osm.pbf
	@echo "Building MBTiles $(basename $@)"
	cp $(basename $@).osm.pbf mbtiles_build/data.osm.pbf
	docker build ./mbtiles_build --tag headway_mbtiles_builder
	bash -c 'export CID=$$(docker create headway_mbtiles_builder) && \
		docker cp $$CID:/data/output.mbtiles $@ && \
		docker rm -v $$CID'

%.nominatim.sql %.nominatim_tokenizer.tgz: %.osm.pbf
	@echo "Building geocoding index for $(basename $(basename $@))."
	cp $(basename $(basename $@)).osm.pbf ./geocoder/nominatim_build/data.osm.pbf
	docker build ./geocoder/nominatim_build --tag headway_nominatim_build
	bash -c 'export CID=$$(docker create headway_nominatim_build) && \
		docker cp $$CID:/dump/nominatim.sql $(basename $(basename $@)).nominatim.sql && \
		docker cp $$CID:/nominatim/tokenizer.tgz $(basename $(basename $@)).nominatim_tokenizer.tgz && \
		docker rm -v $$CID'

%.photon.tgz: %.nominatim.sql
	@echo "Importing data into photon and building index for $(basename $(basename $@))."
	cp $(basename $(basename $@)).nominatim.sql ./geocoder/photon_build/data.nominatim.sql
	docker build ./geocoder/photon_build --tag headway_photon_build
	bash -c 'export CID=$$(docker create headway_photon_build) && \
		docker cp $$CID:/photon/photon.tgz $@ && \
		docker rm -v $$CID'

%.graph.obj: %.osm.pbf %.gtfs.tar
	@echo "Building OpenTripPlanner graph for $*."
	cp $*.osm.pbf ./otp/build/data.osm.pbf
	cp $*.gtfs.tar ./otp/build/gtfs.tar
	set -e ;\
		ITAG=headway_build_otp_$$(echo $(notdir $*) | tr '[:upper:]' '[:lower:]') ;\
		docker build ./otp/build --tag $${ITAG} ;\
		CID=$$(docker create $${ITAG}) ;\
		docker cp $$CID:/data/graph.obj $@ ;\
		docker rm -v $$CID

%.valhalla.tar: %.osm.pbf
	@echo "Building Valhalla tiles for $(basename $(basename $@))."
	cp $< ./valhalla/build/data.osm.pbf
	set -e ;\
		ITAG=headway_build_valhalla_$$(echo $(notdir $*) | tr '[:upper:]' '[:lower:]') ;\
		docker build ./valhalla/build --tag $${ITAG} ;\
		CID=$$(docker create $${ITAG}) ;\
		docker cp $$CID:/tiles/valhalla.tar $@ ;\
		docker rm -v $$CID

%.gtfs.tar:
	set -e ;\
		ITAG=headway_build_gtfs_$$(echo $(notdir $*) | tr '[:upper:]' '[:lower:]') ;\
		docker build ./gtfs --build-arg CITY_NAME=$(notdir $*) --tag $${ITAG} ;\
		CID=$$(docker create $${ITAG}) ;\
		docker cp $$CID:/gtfs_feeds/gtfs.tar $@ ;\
		docker rm -v $$CID

tileserver_image: %.mbtiles
	@echo "Building tileserver image for $(basename $@)."
	cp $(basename $@).mbtiles ./tileserver/tiles.mbtiles
	docker build ./tileserver --tag headway_tileserver

nominatim_image:
	@echo "Building nominatim image"
	docker build ./geocoder/nominatim --tag headway_nominatim

photon_image:
	@echo "Building photon image"
	docker build ./geocoder/photon --tag headway_photon

nginx_image:
	docker build ./web --tag headway_nginx

otp_image:
	docker build ./otp/run --tag headway_otp

valhalla_image:
	docker build ./valhalla/run --tag headway_valhalla

tag_images: nginx_image photon_image nominatim_image otp_image valhalla_image
	@echo "Tagged images"

$(filter %,$(CITIES)): %: ${DATA_DIR}/%.osm.pbf ${DATA_DIR}/%.nominatim.sql ${DATA_DIR}/%.nominatim_tokenizer.tgz ${DATA_DIR}/%.photon.tgz ${DATA_DIR}/%.mbtiles ${DATA_DIR}/%.graph.obj ${DATA_DIR}/%.gtfs.tar ${DATA_DIR}/%.valhalla.tar ${DATA_DIR}/%.bbox tag_images
	@echo "Built $@"

# Clean only generated data.
clean:
	rm -rf ${DATA_DIR}/*.mbtiles
	rm -rf ${DATA_DIR}/*.nominatim.sql
	rm -rf ${DATA_DIR}/*.nominatim_tokenizer.tgz
	rm -rf ${DATA_DIR}/*.photon.tgz
	rm -rf ${DATA_DIR}/*.graph.obj
	rm -rf ${DATA_DIR}/bbox.txt
	rm -rf ${DATA_DIR}/bbox.txt.bak

%.up: %
	docker-compose kill || echo "Containers not up"
	docker-compose down || echo "Containers dont exist"
	docker-compose up -d

# Clean even the data we have to download from external sources.
clean_all: clean
	rm -rf ${DATA_DIR}/*.osm.pbf
	rm -rf ${DATA_DIR}/*.gtfs.tar
	rm -rf ${DATA_DIR}/sources
