// A generated module for Headway functions
//
// This module has been generated via dagger init and serves as a reference to
// basic module structure as you get started with Dagger.
//
// Two functions have been pre-created. You can modify, delete, or add to them,
// as needed. They demonstrate usage of arguments and return types using simple
// echo and grep commands. The functions can be called from the dagger CLI or
// from one of the SDKs.
//
// The first line in this comment block is a short description line and the
// rest is a long description with more detail on the module's purpose or usage,
// if appropriate. All modules should have a short description.

package main

import (
	"context"
	"dagger/headway/internal/dagger"
	"fmt"
	"strconv"
	"strings"
	"time"
)

type Headway struct {
	Area        string
	OSMExport   *OSMExport
	ServicesDir *dagger.Directory
}

type Bbox struct {
	Left   float64
	Bottom float64
	Right  float64
	Top    float64
}

func (b *Bbox) CommaSeparated() string {
	return fmt.Sprintf("%f,%f,%f,%f", b.Left, b.Bottom, b.Right, b.Top)
}

func ParseBboxStr(bboxStr string) (*Bbox, error) {
	parts := strings.Fields(strings.TrimSpace(bboxStr))
	if len(parts) != 4 {
		return nil, fmt.Errorf("invalid bbox format")
	}
	left, err := strconv.ParseFloat(parts[0], 64)
	if err != nil {
		return nil, fmt.Errorf("failed to parse left: %w", err)
	}
	bottom, err := strconv.ParseFloat(parts[1], 64)
	if err != nil {
		return nil, fmt.Errorf("failed to parse bottom: %w", err)
	}
	right, err := strconv.ParseFloat(parts[2], 64)
	if err != nil {
		return nil, fmt.Errorf("failed to parse right: %w", err)
	}
	top, err := strconv.ParseFloat(parts[3], 64)
	if err != nil {
		return nil, fmt.Errorf("failed to parse top: %w", err)
	}

	result := Bbox{Left: left, Bottom: bottom, Right: right, Top: top}
	return &result, nil
}

type OSMExport struct {
	// PBF file
	File *dagger.File
}

// Downloads OSM extract from bbike
func (h *Headway) New(
	// Area name (e.g. "Seattle")
	area string,

	// Local OSM PBF file to mount, if missing will download based on area
	// +defaultPath=""
	localPbf *dagger.File,

	// +defaultPath="./services"
	servicesDir *dagger.Directory) *Headway {
	h.Area = area
	h.ServicesDir = servicesDir
	if localPbf == nil {
		h.OSMExport = h.DownloadPBF(area)
	} else {
		h.OSMExport = h.LocalPBF(localPbf)
	}

	return h
}

func (h *Headway) ServiceDir(subDirectory string) *dagger.Directory {
	return h.ServicesDir.Directory(subDirectory)
}

/**
 * Full build
 */
func (h *Headway) Build(ctx context.Context,
	// +defaultPath="./services"
	servicesDir *dagger.Directory) (*dagger.Directory, error) {

	if h.Area == "" {
		return nil, fmt.Errorf("Area is required")
	}

	output := dag.Directory()

	// BUILD +save-extract --area=${area}
	output = output.WithFile(h.Area+".osm.pbf", h.OSMExport.File)

	// BUILD +save-mbtiles --area=${area}
	mbtiles, err := h.Mbtiles(ctx, false, servicesDir.Directory("tilebuilder"))
	if err != nil {
		return nil, fmt.Errorf("failed to build mbtiles: %w", err)
	}
	// area.mbtils
	output = output.WithFile(h.Area+".mbtiles", mbtiles)

	// TODO
	// IF [ ! -z "${transit_feeds}" ]
	// BUILD +save-gtfs --area=${area} --transit_feeds=${transit_feeds}
	// BUILD +save-otp-graph --area=${area} --transit_feeds=${transit_feeds} --clip_to_gtfs=0
	// BUILD +save-transit-elevations -graph --area=${area} --transit_feeds=${transit_feeds} --clip_to_gtfs=0
	// END

	// BUILD +save-valhalla --area=${area}
	valhalla := h.ValhallaTiles()
	output = output.WithFile(h.Area+".valhalla.tar.zst", compressDir(valhalla))

	// BUILD +save-pelias --area=${area} --countries=${countries}
	//     BUILD +save-pelias-config --area=${area} --countries=${countries}
	pelias := h.PeliasConfig(ctx, h.Area, []string{}, servicesDir.Directory("pelias"))
	output = output.WithFile(h.Area+".pelias.json", pelias.Config)

	//     BUILD +save-elasticsearch --area=${area} --countries=${countries}
	elasticSearch := pelias.PeliasElasticsearchData(ctx)
	output = output.WithFile(h.Area+".elasticsearch.tar.zst", compressDir(elasticSearch))

	//     BUILD +save-placeholder --area=${area} --countries=${countries}
	placeholder := pelias.PeliasPreparePlaceholder(ctx)
	output = output.WithFile(h.Area+".placeholder.tar.zst", compressDir(placeholder))

	// BUILD +save-tileserver-terrain
	terrain, err := h.TileserverTerrain(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to download tileserver terrain: %w", err)
	}
	output = output.WithFile("terrain.mbtiles", terrain.File("terrain.mbtiles"))
	output = output.WithFile("landcover.mbtiles", terrain.File("landcover.mbtiles"))

	return output, nil
}

func (h *Headway) BuildTransit(ctx context.Context,
	transitConfigDir *dagger.Directory) (*dagger.Directory, error) {

	output := dag.Directory()

	transitFeedsDir := transitConfigDir.Directory("gtfs-feeds")

	otpBuildConfig := (*dagger.File)(nil)
	otpConfigExists, err := transitFeedsDir.Exists(ctx, "otp-build-config.json")
	if err != nil {
		panic(fmt.Errorf("failed to check if otp-build-config.json exists: %w", err))
	}
	if otpConfigExists {
		otpBuildConfig = transitConfigDir.File("otp-build-config.json")
	}
	elevations := dag.Directory()
	// TODO: ARG otp_build_config
	transitFeedsFiles, err := transitFeedsDir.Entries(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to get entries in transit feeds dir: %w", err))
	}
	for _, entry := range transitFeedsFiles {
		transitFeedsFile := transitFeedsDir.File(entry)
		zone := h.TransitZone(ctx, transitFeedsFile)
		if otpBuildConfig != nil {
			zone.WithOtpBuildConfig(ctx, otpBuildConfig)
		}
		zone = zone.WithGtfsDir(ctx, zone.BuildGtfsDir(ctx))
		output = output.WithFile(fmt.Sprintf("%s.gtfs.tar.zst", zone.Name(ctx)), compressDir(zone.GTFSDir))
		// TODO: make an arg or config or something... or just always clip?
		// Any harm besides slowing things down a little?
		// In practice it seems pretty quick compared to the subsequent work done with the output,
		// so maybe it's not worth complicating things here.
		clipToGtfs := true
		otpGraph := zone.OtpGraph(ctx, clipToGtfs)
		output = output.WithFile(fmt.Sprintf("%s.graph.obj.zst", zone.Name(ctx)), compressFile(otpGraph))
		elevations = elevations.WithDirectory("./", zone.Elevations(ctx))
		// TODO: What is the `-graph` about?
		// BUILD +save-transit-elevations -graph --area=${area} --transit_feeds=${transit_feeds} --clip_to_gtfs=0
	}
	output = output.WithDirectory("elevations", elevations)

	return output, nil
}

type TransitZone struct {
	Headway        *Headway
	CacheKey       string
	TransitFeeds   *dagger.File
	GTFSDir        *dagger.Directory
	OSMExport      *OSMExport
	OTPBuildConfig *dagger.File
}

func (h *Headway) TransitZone(ctx context.Context, transitFeeds *dagger.File) *TransitZone {
	cacheKey := time.Now().Format("2006-01-02")
	return &TransitZone{
		Headway:      h,
		CacheKey:     cacheKey,
		TransitFeeds: transitFeeds,
	}
}

func (t *TransitZone) ZoneName(ctx context.Context) string {
	fileName, err := t.TransitFeeds.Name(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to get transit feeds name: %w", err))
	}
	return strings.TrimSuffix(fileName, ".gtfs_feeds.csv")
}

func (t *TransitZone) WithOtpBuildConfig(ctx context.Context, otpBuildConfig *dagger.File) *TransitZone {
	t.OTPBuildConfig = otpBuildConfig
	return t
}

func (t *TransitZone) Name(ctx context.Context) string {
	return fmt.Sprintf("%s-%s-%s", t.Headway.Area, t.ZoneName(ctx), t.CacheKey)
}

func (t *TransitZone) ClippedOsmExport(ctx context.Context) *OSMExport {
	bbox, err := t.BBox(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to get bbox: %w", err))
	}

	return t.Headway.OSMExport.Clip(ctx, bbox)
}

func (t *TransitZone) OtpGraph(ctx context.Context, clipToGtfs bool) *dagger.File {

	// FROM +save-base
	// ARG --required area
	// ARG --required transit_feeds
	// ARG --required clip_to_gtfs
	// ARG otp_build_config

	// # When working with a very large (e.g. planet sized) osm.pbf, we can't support
	// # transit for the entire thing, but we can support smaller transit zones within the
	// # planet.
	// # We extract a bbox'd area of the input osm.pbf around the actual transit
	// # zone for OTP to have any chance of fitting into memory.
	// ARG transit_zone=$(basename $transit_feeds .gtfs_feeds.csv)
	// IF [ -n "$clip_to_gtfs" ]
	//     ARG output_name="${area}-${transit_zone}"
	// ELSE
	//     ARG clip_to_gtfs=0
	//     ARG output_name="${transit_zone}"
	// END
	osmExport := t.Headway.OSMExport
	if clipToGtfs {
		osmExport = t.ClippedOsmExport(ctx)
	}

	// COPY +cache-buster/todays_date .
	// ARG cache_key=$(cat todays_date)

	// COPY (+otp-build-graph/graph.obj --area=${area} \
	//                            --clip_to_gtfs=${clip_to_gtfs} \
	//                            --transit_feeds=${transit_feeds} \
	//                            --cache_key=${cache_key} \
	//                            --otp_build_config=${otp_build_config} \
	// ) /graph.obj

	if t.GTFSDir == nil {
		panic("TransitZone.GTFSDir must be set to build OTP graph, call `WithGTFSDir` first")
	}

	container := dag.Container().
		From("opentripplanner/opentripplanner:2.7.0").
		WithExec([]string{"mkdir", "/var/opentripplanner"}).
		WithWorkdir("/var/opentripplanner")

	if t.OTPBuildConfig != nil {
		container = container.WithMountedFile("/var/opentripplanner/otp-build-config.json", t.OTPBuildConfig)
	}

	elevationTifs := t.Elevations(ctx)

	container = container.
		WithMountedFile("/var/opentripplanner/data.osm.pbf", osmExport.File).
		WithMountedDirectory("/var/opentripplanner", t.GTFSDir).
		WithDirectory("/var/opentripplanner", elevationTifs)

	return container.
		WithExec([]string{"--build", "--save"}, dagger.ContainerWithExecOpts{UseEntrypoint: true}).
		File("/var/opentripplanner/graph.obj")
}

func (o *OSMExport) Clip(ctx context.Context, bbox *Bbox) *OSMExport {
	container := WithAptPackages(slimContainer(), "osmium-tool").
		WithExec([]string{"mkdir", "-p", "/app"}).
		WithMountedFile("/app/data.osm.pbf", o.File).
		WithExec([]string{"osmium", "extract", "--bbox", bbox.CommaSeparated(), "--output", "/app/clipped.osm.pbf", "/app/data.osm.pbf"})

	return &OSMExport{File: container.File("/app/clipped.osm.pbf")}
}

func (t *TransitZone) WithGtfsDir(ctx context.Context, gtfsDir *dagger.Directory) *TransitZone {
	t.GTFSDir = gtfsDir
	return t
}

func (t *TransitZone) BuildGtfsDir(ctx context.Context) *dagger.Directory {
	servicesDir := t.Headway.ServicesDir.Directory("gtfs")

	assumeBikesAllowed := t.Headway.Gtfout(ctx).File("assume-bikes-allowed")

	container := dag.Container().
		From("python:3")
	return WithAptPackages(container, "zip").
		WithExec([]string{"pip", "install", "requests"}).
		WithMountedDirectory("/app", servicesDir).
		WithWorkdir("/app").
		WithMountedFile("/usr/local/bin/assume-bikes-allowed", assumeBikesAllowed).
		WithMountedFile("gtfs_feeds.csv", t.TransitFeeds).
		WithExec([]string{"sh", "-c", "./download_gtfs_feeds.py --output=downloaded < gtfs_feeds.csv"}).
		WithExec([]string{"sh", "-c", "./build_gtfs.sh --input downloaded --output ./output"}).
		Directory("./output")
}

/**
 * TileServer
 */

// Downloads terrain tiles from headway-data repository
func (h *Headway) TileserverTerrain(ctx context.Context) (*dagger.Directory, error) {
	assetRoot := "https://github.com/headwaymaps/headway-data/raw/main/tiles/"

	container := downloadContainer().
		WithExec([]string{"wget", "-nv", assetRoot + "terrain.mbtiles"}).
		WithExec([]string{"wget", "-nv", assetRoot + "landcover.mbtiles"})

	return container.Directory("/data"), nil
}

// Build assets for the tileserver
func (h *Headway) TileserverAssets(ctx context.Context,
	// +defaultPath="./services/tileserver"
	serviceDir *dagger.Directory) *dagger.Directory {
	container := dag.Container().
		From("rust:bookworm")

	assetsDir := serviceDir.Directory("assets")
	container = WithAptPackages(container, "libfreetype6-dev").
		WithMountedDirectory("/app/assets/", assetsDir).
		WithExec([]string{"cargo", "install", "spreet", "build_pbf_glyphs"}).

		// FONTS
		WithExec([]string{"build_pbf_glyphs", "/app/assets/fonts", "/output/fonts"}).

		// SPRITES
		WithExec([]string{"mkdir", "-p", "/output/sprites"}).
		WithExec([]string{"spreet", "/app/assets/sprites", "/output/sprites/sprite"}).
		WithExec([]string{"spreet", "--retina", "/app/assets/sprites", "/output/sprites/sprite@2x"})

	return container.Directory("/output")
}

// Build tileserver init container image
func (h *Headway) TileserverInitImage(ctx context.Context,
	// +defaultPath="./services/tileserver"
	serviceDir *dagger.Directory,
) *dagger.Container {
	return downloadContainer().
		WithFile("/app/init.sh", serviceDir.File("init.sh")).
		WithDefaultArgs([]string{"/app/init.sh"})
}

func (h *Headway) TileserverServeImage(ctx context.Context,
	// +defaultPath="./services/tileserver"
	serviceDir *dagger.Directory,
) *dagger.Container {
	container := dag.Container().
		From("node:20-slim").
		WithExec([]string{"npm", "install", "-g", "tileserver-gl-light"})

	builtAssets := h.TileserverAssets(ctx, serviceDir)

	container = container.WithExec([]string{"mkdir", "-p", "/app/styles"}).
		WithExec([]string{"chown", "-R", "node", "/app"}).
		WithDirectory("/app/fonts", builtAssets.Directory("fonts")).
		WithDirectory("/app/sprites", builtAssets.Directory("sprites")).
		WithDirectory("/app/styles/basic", serviceDir.Directory("styles/basic")).
		WithDirectory("/templates/", serviceDir.Directory("templates")).
		WithFile("/app/configure_run.sh", serviceDir.File("configure_run.sh")).
		WithEnvVariable("HEADWAY_PUBLIC_URL", "http://127.0.0.1:8080").
		WithDefaultArgs([]string{"/app/configure_run.sh"})

	return container
}

// FIXME: ExportImage doesn't work.  (but dagger shell: export-image does work!)
func (h *Headway) ExportTileserverInitImage(ctx context.Context,
	// +defaultPath="./services/tileserver"
	serviceDir *dagger.Directory,
	tags []string,
) error {
	container := h.TileserverInitImage(ctx, serviceDir)
	return h.ExportContainerImage(ctx, container, "tileserver-init", tags)
}

// Builds mbtiles using Planetiler
func (h *Headway) Mbtiles(ctx context.Context,
	// +optional
	// Whether this is a planet-scale build (affects memory settings)
	isPlanetBuild bool,
	// +defaultPath="./services/tilebuilder"
	serviceDir *dagger.Directory) (*dagger.File, error) {

	if h.OSMExport == nil || h.OSMExport.File == nil {
		panic("Headway.OSMExport.File must be set to build mbtiles")
	}

	// memoryScript := serviceDir.File("percent-of-available-memory")
	container := dag.Container().
		From("ghcr.io/onthegomap/planetiler:0.7.0").
		WithExec([]string{"mkdir", "-p", "/data/sources"}).
		WithExec([]string{"sh", "-c", "curl --no-progress-meter https://data.maps.earth/planetiler_fixtures/sources.tar | tar -x --directory /data/sources"}).
		WithMountedFile("/data/data.osm.pbf", h.OSMExport.File)

	entrypoint, err := container.Entrypoint(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get entrypoint: %w", err)
	}

	if isPlanetBuild {
		container = container.WithExec(append(entrypoint,
			"--osm_path=/data/data.osm.pbf",
			"--force",
			"--bounds=planet",
			"--nodemap-type=array",
			"--storage=mmap",
			"-Xmx$(/percent-of-available-memory 75)",
			"-XX:MaxHeapFreeRatio=40",
		))
	} else {
		container = container.WithExec(append(entrypoint, "--osm_path=/data/data.osm.pbf", "--force"))
	}

	return container.File("/data/output.mbtiles"), nil
}

/**
 * Pelias
 */

type Pelias struct {
	Config     *dagger.File
	ServiceDir *dagger.Directory
	Headway    *Headway
}

// We use this both for import and for production pelias instances.
// But we might want to try a longer timeout for the import process?
func (h *Headway) PeliasConfig(ctx context.Context,
	area string,
	// +optional
	countries []string,
	// +defaultPath="./services/pelias"
	serviceDir *dagger.Directory,
) *Pelias {
	countriesStr := strings.Join(countries, ",")
	config := dag.Container().
		From("node:20-slim").
		WithDirectory("generate_config", serviceDir.Directory("generate_config")).
		WithWorkdir("generate_config").
		WithExec([]string{"yarn", "install"}).
		WithExec([]string{"yarn", "build"}).
		WithExec([]string{"sh", "-c", fmt.Sprintf("bin/generate-pelias-config areas.csv '%s' '%s' > pelias.json", area, countriesStr)}).
		File("pelias.json")

	return &Pelias{Config: config, ServiceDir: serviceDir, Headway: h}
}

func (p *Pelias) PeliasContainerFrom(containerName string) *dagger.Container {
	container := dag.Container().
		From(containerName).
		WithMountedDirectory("/pelias-service", p.ServiceDir).
		WithFile("/code/pelias.json", p.Config)
	return container
}

func (p *Pelias) PeliasDownloadWhosOnFirst() *dagger.Directory {
	container := p.PeliasContainerFrom("pelias/whosonfirst:master").
		WithExec([]string{"./bin/download"})
	return container.Directory("/data/whosonfirst")
}

func (p *Pelias) PeliasDownloadOpenAddresses() *dagger.Directory {
	container := p.PeliasContainerFrom("pelias/openaddresses:master").
		WithExec([]string{"./bin/download"})
	return container.Directory("/data/openaddresses")
}

func (p *Pelias) PeliasPreparePlaceholder(ctx context.Context) *dagger.Directory {
	container := p.PeliasContainerFrom("pelias/placeholder:master").
		WithMountedDirectory("/data/whosonfirst", p.PeliasDownloadWhosOnFirst()).
		WithExec([]string{"bash", "-c", "./cmd/extract.sh && ./cmd/build.sh"})
	return container.Directory("/data/placeholder")
}

type PeliasImporter struct {
	Pelias               *Pelias
	ElasticsearchCache   *dagger.CacheVolume
	ElasticsearchService *dagger.Service
}

func (p *Pelias) PeliasImporter() *PeliasImporter {
	cacheBuster := time.Now().UnixNano()
	elasticsearchCache := dag.CacheVolume(fmt.Sprintf("pelias-elasticsearch-%d", cacheBuster))

	// REVIEW: Sharing? Would "PRIVATE" allow us to get rid of the cache buster?
	// maybe cache buster should be based on the input not timestamp
	opts := dagger.ContainerWithMountedCacheOpts{Owner: "elasticsearch", Sharing: "SHARED"}

	// NOTE: docker-compose passes some extra arguments to this container, e.g. IPC and mem size
	db := dag.Container().
		From("pelias/elasticsearch:8.12.2-beta").
		WithExposedPort(9200).
		WithMountedCache("/usr/share/elasticsearch/data", elasticsearchCache, opts).
		AsService()

	return &PeliasImporter{
		Pelias:               p,
		ElasticsearchCache:   elasticsearchCache,
		ElasticsearchService: db,
	}
}

func (p *PeliasImporter) PeliasImporterContainerFrom(containerName string) *dagger.Container {
	return p.Pelias.PeliasContainerFrom(containerName).
		WithServiceBinding("pelias-elasticsearch", p.ElasticsearchService).
		WithExec([]string{"/pelias-service/wait.sh"})
}

func (p *PeliasImporter) PeliasImportSchema() *dagger.Container {
	return p.PeliasImporterContainerFrom("pelias/schema:master").
		WithExec([]string{"./bin/create_index"})
}

func (p *PeliasImporter) PeliasImportWhosOnFirst() *dagger.Container {
	return p.PeliasImporterContainerFrom("pelias/whosonfirst:master").
		WithMountedDirectory("/data/whosonfirst", p.Pelias.PeliasDownloadWhosOnFirst()).
		WithExec([]string{"./bin/start"})
}

func (p *PeliasImporter) PeliasImportOpenAddresses() *dagger.Container {
	return p.PeliasImporterContainerFrom("pelias/openaddresses:master").
		WithMountedDirectory("/data/openaddresses", p.Pelias.PeliasDownloadOpenAddresses()).
		// OpenAddress import also uses WhosOnFirst data
		WithMountedDirectory("/data/whosonfirst", p.Pelias.PeliasDownloadWhosOnFirst()).
		WithExec([]string{"./bin/start"})
}

func (p *PeliasImporter) PeliasImportOpenStreetMap() *dagger.Container {
	if p.Pelias.Headway.OSMExport == nil || p.Pelias.Headway.OSMExport.File == nil {
		panic("PeliasImporter: Headway.OSMExport.File must be set to import OpenStreetMap data")
	}

	return p.PeliasImporterContainerFrom("pelias/openstreetmap:master").
		WithMountedFile("/data/openstreetmap/data.osm.pbf", p.Pelias.Headway.OSMExport.File).
		// OpenStreetMap import also uses WhosOnFirst data
		WithMountedDirectory("/data/whosonfirst", p.Pelias.PeliasDownloadWhosOnFirst()).
		WithExec([]string{"./bin/start"})
}

func (p *PeliasImporter) PeliasImportPolylines() *dagger.Container {
	return p.PeliasImporterContainerFrom("pelias/polylines:master").
		WithMountedFile("/data/polylines/extract.0sv", p.Pelias.Headway.ValhallaPolylines()).
		// Polylines import also uses WhosOnFirst data
		WithMountedDirectory("/data/whosonfirst", p.Pelias.PeliasDownloadWhosOnFirst()).
		WithExec([]string{"./bin/start"})
}

func (p *Pelias) PeliasElasticsearchData(ctx context.Context) *dagger.Directory {
	err := error(nil)

	importer := p.PeliasImporter()

	_, err = importer.PeliasImportSchema().
		Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to import pelias schema: %w", err))
	}

	_, err = importer.PeliasImportWhosOnFirst().
		Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to import WhoseOnFirst data: %w", err))
	}

	_, err = importer.PeliasImportOpenAddresses().
		Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to import OpenAddresses data: %w", err))
	}

	_, err = importer.PeliasImportOpenStreetMap().
		Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to import OpenStreetMap data: %w", err))
	}

	_, err = importer.PeliasImportPolylines().
		Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to import OpenStreetMap data: %w", err))
	}

	opts := dagger.ContainerWithMountedCacheOpts{Owner: "elasticsearch", Sharing: "SHARED"}
	return dag.Container().
		From("debian:bookworm-slim").
		// The cache "Owner" is two things:
		//    1. the owner on the filesystem (as in `chown $owner`)
		//    2. a namespace within the cache, so the same cache will container different data depending on the Owner argument
		//
		// We need the "elasticsearch" cache, but mounting will error if the user doesn't exist, so we add the user
		WithExec([]string{"useradd", "elasticsearch"}).
		WithMountedCache("/data-cache", importer.ElasticsearchCache, opts).
		WithExec([]string{"cp", "-r", "/data-cache", "/export"}, dagger.ContainerWithExecOpts{UseEntrypoint: false}).
		Directory("/export")
}

/**
 * Valhalla
 */
func valhallaBaseContainer() *dagger.Container {
	return dag.Container().
		From("ghcr.io/valhalla/valhalla:latest").
		WithWorkdir("/tiles")
}

func valhallaBuildContainer() *dagger.Container {
	return valhallaBaseContainer().
		WithExec([]string{"sh", "-c", "valhalla_build_config --mjolnir-tile-dir /tiles --mjolnir-timezone /tiles/timezones.sqlite --mjolnir-admin /tiles/admins.sqlite > valhalla.json"}).
		WithExec([]string{"sh", "-c", "valhalla_build_timezones > /tiles/timezones.sqlite"})
}

// Builds Valhalla routing tiles
func (h *Headway) ValhallaTiles() *dagger.Directory {
	if h.OSMExport == nil || h.OSMExport.File == nil {
		panic("Headway.OSMExport.File must be set to build Valhalla tiles")
	}

	container := valhallaBuildContainer().
		WithMountedFile("/data/osm/data.osm.pbf", h.OSMExport.File).
		WithExec([]string{"valhalla_build_tiles", "-c", "valhalla.json", "/data/osm/data.osm.pbf"})

	return container.Directory("/tiles")
}

func (h *Headway) ValhallaPolylines() *dagger.File {
	container := valhallaBuildContainer().
		WithMountedDirectory("/tiles", h.ValhallaTiles()).
		WithExec([]string{"sh", "-c", "valhalla_export_edges -c valhalla.json > /tiles/polylines.0sv"})

	return container.File("/tiles/polylines.0sv")
}

// Extracts bounding box for a given area from bboxes.csv
func (h *Headway) BBox(ctx context.Context) (*Bbox, error) {
	bboxesFile := h.ServiceDir("gtfs").File("bboxes.csv")

	// Area name to look up (must exist in bboxes.csv)
	area := h.Area
	if area == "" {
		return nil, fmt.Errorf("Area is required to get bounding box")
	}

	container := dag.Container().
		From("debian:bookworm-slim").
		WithMountedFile("/bboxes.csv", bboxesFile).
		WithExec([]string{"sh", "-c", fmt.Sprintf("test $(grep '%s:' /bboxes.csv | wc -l) -eq 1", area)}).
		WithExec([]string{"sh", "-c", fmt.Sprintf("grep '%s:' /bboxes.csv | cut -d':' -f2", area)})

	bboxStr, err := container.Stdout(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get bbox for area %s: %w", area, err)
	}
	return ParseBboxStr(bboxStr)
}

func (t *TransitZone) BBox(ctx context.Context) (*Bbox, error) {
	container := dag.Container().
		From("debian:bookworm-slim")
	container = WithAptPackages(container, "unzip").
		WithMountedFile("/usr/local/bin/gtfs-bbox", t.Headway.Gtfout(ctx).File("gtfs-bbox")).
		WithExec([]string{"mkdir", "-p", "/app"}).
		WithExec([]string{"mkdir", "-p", "/app/gtfs"}).
		WithWorkdir("/app").
		WithMountedDirectory("/app/gtfs_zips", t.GTFSDir).
		WithExec([]string{"sh", "-c", "cd gtfs_zips && ls *.zip | while read zip_file; do unzip -d ../gtfs/$(basename $zip_file .zip) $zip_file; done"}).
		WithExec([]string{"sh", "-c", "gtfs-bbox gtfs/*"})

	bboxStr, err := container.Stdout(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get bbox for transit zone %s: %w", t.Name(ctx), err)
	}
	return ParseBboxStr(bboxStr)
}

// Downloads GTFS mobility database CSV
func (h *Headway) GtfsGetMobilitydb(ctx context.Context) *dagger.File {
	return downloadFile("https://storage.googleapis.com/storage/v1/b/mdb-csv/o/sources.csv?alt=media")
}

// Builds Rust GTFS processing tools
// I'm not yet sure how exporting will work in situ. Something akin to:
//
//	dagger -c 'gtfout | file assume-bikes-allowed | export ./assume-bikes-allowed'
func (h *Headway) Gtfout(ctx context.Context) *dagger.Directory {
	sourceDir := h.ServicesDir.Directory("gtfs/gtfout")
	container := dag.Container().
		From("rust:bookworm").
		WithMountedDirectory("/gtfout", sourceDir).
		WithWorkdir("/gtfout").
		WithExec([]string{"cargo", "build", "--release"})

	return container.Directory("/gtfout/target/release")
}

// Converts elevation HGT files to TIF format
func (t *TransitZone) Elevations(ctx context.Context) *dagger.Directory {
	bbox, err := t.BBox(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to get bounding box: %w", err))
	}
	return elevations(ctx, bbox, t.Headway.ServicesDir)
}

func (h *Headway) Elevations(ctx context.Context) *dagger.Directory {
	bbox, err := h.BBox(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to get bounding box: %w", err))
	}
	return elevations(ctx, bbox, h.ServicesDir)
}

func elevations(ctx context.Context, bbox *Bbox, serviceDirectory *dagger.Directory) *dagger.Directory {
	elevationHgts := valhallaBaseContainer().
		WithExec([]string{"valhalla_build_elevation", "--outdir", "elevation-hgts", "--from-bbox=" + bbox.CommaSeparated()}).
		Directory("/tiles/elevation-hgts")

	// Convert elevation HGT files to TIF format
	demScript := serviceDirectory.Directory("otp").File("dem-hgt-to-tif")
	container := dag.Container().
		From("debian:bookworm-slim")
	container = WithAptPackages(container, "gdal-bin").
		WithMountedFile("/dem-hgt-to-tif", demScript).
		WithMountedDirectory("/elevation-hgts", elevationHgts).
		WithExec([]string{"/dem-hgt-to-tif", "/elevation-hgts", "/elevation-tifs"})

	return container.Directory("/elevation-tifs")
}

/**
 * OSM PBF
 */

// Downloads OSM extract from bbike
func (h *Headway) DownloadPBF(
	// Area name (e.g. "Seattle")
	area string) *OSMExport {

	downloadUrl := fmt.Sprintf("https://download.bbbike.org/osm/bbbike/%s/%s.osm.pbf", area, area)
	pbf := downloadFile(downloadUrl)

	return &OSMExport{File: pbf}
}

// Mounts a local OSM PBF file
func (h *Headway) LocalPBF(
	// Local OSM PBF file to mount
	pbfFile *dagger.File) *OSMExport {
	return &OSMExport{
		File: pbfFile,
	}
}

/**
 * Helpers
 */

func slimContainer() *dagger.Container {
	return dag.Container().
		From("debian:bookworm-slim")
}

// Export the given container
func (h *Headway) ExportContainerImage(ctx context.Context,
	container *dagger.Container,
	imageName string,
	tags []string,
) error {
	// CURRENTLY NOT WORKING
	// Maybe I need to mount my local docker socket?
	for _, tag := range tags {
		err := container.ExportImage(ctx, "ghcr.io/headwaymaps/"+imageName+":"+tag)
		if err != nil {
			return fmt.Errorf("failed to export image with tag %s: %w", tag, err)
		}
	}
	return nil
}

func downloadContainer() *dagger.Container {
	container := dag.Container().
		From("debian:bookworm-slim").
		WithWorkdir("/data")
	container = WithAptPackages(container, "wget", "ca-certificates", "zstd")
	return container
}

func downloadFile(url string) *dagger.File {
	container := downloadContainer().
		WithExec([]string{"wget", "-nv", "-U", "headway/1.0", "-O", "/data/file", url})
	return container.File("/data/file")
}

// Returns a container with the specified apt packages installed
func WithAptPackages(container *dagger.Container, packages ...string) *dagger.Container {
	if len(packages) == 0 {
		println("WithAptPackages: no packages specified, returning original container")
		return container
	}

	pkgList := strings.Join(packages, " ")
	cmd := fmt.Sprintf("apt-get update && apt-get install -y --no-install-recommends %s && rm -rf /var/lib/apt/lists/*", pkgList)

	return container.WithExec([]string{"sh", "-c", cmd})
}

func compressDir(dir *dagger.Directory) *dagger.File {
	container := WithAptPackages(slimContainer(), "zstd").
		WithExec([]string{"mkdir", "/app"}).
		WithWorkdir("/app").
		WithMountedDirectory("/app/input", dir).
		WithExec([]string{"sh", "-c", "tar --use-compress-program='zstd -T0' -cf output.tar.zst -C input ."})

	return container.File("output.tar.zst")
}

func compressFile(input *dagger.File) *dagger.File {
	container := WithAptPackages(slimContainer(), "zstd").
		WithExec([]string{"mkdir", "/app"}).
		WithWorkdir("/app").
		WithMountedFile("/app/input", input).
		WithExec([]string{"sh", "-c", "zstd -T0 input"})

	return container.File("input.zst")
}

func (h *Headway) TestCompression(ctx context.Context, dir *dagger.Directory) *dagger.File {
	return compressDir(dir)
}
