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
	Area          string
	OSMExport     *OSMExport
	ServicesDir   *dagger.Directory
	IsPlanetBuild bool
	Countries     string
}

type Artifact struct {
	File      *dagger.File
	Directory *dagger.Directory
}

func (a *Artifact) Compress() *dagger.File {
	if a.File != nil {
		if a.Directory != nil {
			panic("Artifact cannot have both File and Directory set")
		}
		return compressFile(a.File)
	} else {
		if a.Directory == nil {
			panic("Artifact must have either File or Directory set")
		}
		return compressDir(a.Directory)
	}
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
	// +optional
	countries string,
	// +defaultPath="./services"
	servicesDir *dagger.Directory) *Headway {
	h.Countries = countries
	h.IsPlanetBuild = countries == "ALL"
	h.ServicesDir = servicesDir
	return h
}

// Downloads OSM extract from bbike
func (h *Headway) WithArea(
	// Area name (e.g. "Seattle")
	area string,

	// Local OSM PBF file to mount, if missing will download based on area
	// +defaultPath=""
	localPbf *dagger.File,
) *Headway {
	h.Area = area
	if localPbf == nil {
		h.OSMExport = h.DownloadPBF(area)
	} else {
		h.OSMExport = h.LocalPBF(localPbf)
	}

	return h
}

func (h *Headway) ServiceDir(subDirectory string) *dagger.Directory {
	if h.ServicesDir == nil {
		panic("Headway.ServicesDir was nil - did you start with `new`?")
	}
	return h.ServicesDir.Directory(subDirectory)
}

/**
 * Full build
 */
func (h *Headway) Build(ctx context.Context) (*dagger.Directory, error) {

	if h.Area == "" {
		return nil, fmt.Errorf("Area is required")
	}

	output := dag.Directory()

	// BUILD +save-extract --area=${area}
	output = output.WithFile(h.Area+".osm.pbf", h.OSMExport.File)

	// BUILD +save-mbtiles --area=${area}
	mbtiles, err := h.Mbtiles(ctx)
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
	valhalla := h.ValhallaTiles(ctx)
	output = output.WithFile(h.Area+".valhalla.tar.zst", valhalla.Compress())

	// BUILD +save-pelias --area=${area} --countries=${countries}
	//     BUILD +save-pelias-config --area=${area} --countries=${countries}
	pelias := h.Pelias(ctx)
	output = output.WithFile(h.Area+".pelias.json", pelias.Config)

	//     BUILD +save-elasticsearch --area=${area} --countries=${countries}
	elasticSearch := pelias.ElasticsearchData(ctx)
	output = output.WithFile(h.Area+".elasticsearch.tar.zst", elasticSearch.Compress())

	//     BUILD +save-placeholder --area=${area} --countries=${countries}
	placeholder := pelias.PreparePlaceholder(ctx)
	output = output.WithFile(h.Area+".placeholder.tar.zst", placeholder.Compress())

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

	// cacheKey := time.Now().Format("2006-01-02")
	cacheKey := time.Now().Format("2006-01")

	output := dag.Directory()

	transitFeedsDir := transitConfigDir.Directory("gtfs-feeds")

	otpBuildConfig := (*dagger.File)(nil)
	otpConfigExists, err := transitConfigDir.Exists(ctx, "otp-build-config.json")
	if err != nil {
		panic(fmt.Errorf("failed to check if otp-build-config.json exists: %w", err))
	}
	if otpConfigExists {
		otpBuildConfig = transitConfigDir.File("otp-build-config.json")
	}
	elevations := dag.Directory()
	transitFeedsFiles, err := transitFeedsDir.Entries(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to get entries in transit feeds dir: %w", err))
	}
	for _, entry := range transitFeedsFiles {
		transitFeedsFile := transitFeedsDir.File(entry)
		zone := h.TransitZone(ctx, transitFeedsFile, cacheKey)
		if otpBuildConfig != nil {
			zone = zone.WithOtpBuildConfig(ctx, otpBuildConfig)
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
	}
	output = output.WithFile(fmt.Sprintf("%s-%s.elevation-tifs.tar.zst", h.Area, cacheKey), compressDir(elevations))

	return output, nil
}

// ===
// Transit
// ===

type TransitZone struct {
	Headway *Headway
	// TODO: verify we're actually caching anything. I think it's primarily the GTFSDir that we want to cache
	CacheKey       string
	TransitFeeds   *dagger.File
	GTFSDir        *dagger.Directory
	OSMExport      *OSMExport
	OTPBuildConfig *dagger.File
}

func (h *Headway) TransitZone(ctx context.Context, transitFeeds *dagger.File, cacheKey string) *TransitZone {
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

// ===
// OpenTripPlanner
// ===
func otpBaseContainer(ctx context.Context) *dagger.Container {
	return dag.Container().
		From("opentripplanner/opentripplanner:2.7.0")
}

func (h *Headway) OtpServeContainer(ctx context.Context) *dagger.Container {
	container := otpBaseContainer(ctx).
		WithExposedPort(8000).
		WithEnvVariable("PORT", "8000").
		WithEntrypoint([]string{"sh", "-c"}).
		WithDefaultArgs([]string{"/docker-entrypoint.sh --load --port ${PORT}"})

	// NOTE: we dropped the healthcheck directive from the old pre-dagger dockerfile
	// because I don't see where dagger supports these kinds of health checks.
	// As I understand it, k8s ignores them anyway
	return container
}

func (h *Headway) OtpInitContainer(ctx context.Context) *dagger.Container {
	return downloadContainer().
		WithFile("/app/init.sh", h.ServiceDir("otp").File("init.sh")).
		WithDefaultArgs([]string{"/app/init.sh"})
}

func (t *TransitZone) OtpGraph(ctx context.Context, clipToGtfs bool) *dagger.File {
	osmExport := t.Headway.OSMExport
	if clipToGtfs {
		osmExport = t.ClippedOsmExport(ctx)
	}

	if t.GTFSDir == nil {
		panic("TransitZone.GTFSDir must be set to build OTP graph, call `WithGTFSDir` first")
	}

	container := otpBaseContainer(ctx).
		WithWorkdir("/var/opentripplanner").
		WithDirectory("/var/opentripplanner", t.GTFSDir).
		WithDirectory("/var/opentripplanner", t.Elevations(ctx)).
		WithMountedFile("/var/opentripplanner/data.osm.pbf", osmExport.File)

	if t.OTPBuildConfig != nil {
		container = container.WithFile("/var/opentripplanner/build-config.json", t.OTPBuildConfig)
	}

	return container.
		WithExec([]string{"--build", "--save"}, dagger.ContainerWithExecOpts{UseEntrypoint: true}).
		File("/var/opentripplanner/graph.obj")
}

func (o *OSMExport) Clip(ctx context.Context, bbox *Bbox) *OSMExport {
	container := slimContainer("osmium-tool").
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
	servicesDir := t.Headway.ServiceDir("gtfs")

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
func (h *Headway) TileserverAssets(ctx context.Context) *dagger.Directory {
	assetsDir := h.ServiceDir("tileserver").Directory("assets")
	container := rustContainer("libfreetype6-dev").
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
func (h *Headway) TileserverInitContainer(ctx context.Context) *dagger.Container {
	return downloadContainer().
		WithFile("/app/init.sh", h.ServiceDir("tileserver").File("init.sh")).
		WithDefaultArgs([]string{"/app/init.sh"})
}

func (h *Headway) TileserverServeContainer(ctx context.Context) *dagger.Container {
	container := slimNodeContainer("gettext-base").
		WithExec([]string{"npm", "install", "-g", "tileserver-gl-light"})

	builtAssets := h.TileserverAssets(ctx)

	container = container.WithExec([]string{"mkdir", "-p", "/app/styles"}).
		WithExec([]string{"chown", "-R", "node", "/app"}).
		WithDirectory("/app/fonts", builtAssets.Directory("fonts")).
		WithDirectory("/app/sprites", builtAssets.Directory("sprites")).
		WithDirectory("/app/styles/basic", h.ServiceDir("tileserver").Directory("styles/basic")).
		WithDirectory("/templates/", h.ServiceDir("tileserver").Directory("templates")).
		WithFile("/app/configure_run.sh", h.ServiceDir("tileserver").File("configure_run.sh")).
		WithEnvVariable("HEADWAY_PUBLIC_URL", "http://127.0.0.1:8080").
		WithDefaultArgs([]string{"/app/configure_run.sh"})

	return container
}

// Builds mbtiles using Planetiler
func (h *Headway) Mbtiles(ctx context.Context) (*dagger.File, error) {

	if h.OSMExport == nil || h.OSMExport.File == nil {
		panic("Headway.OSMExport.File must be set to build mbtiles")
	}

	container := dag.Container().
		From("ghcr.io/onthegomap/planetiler:0.7.0")

	memoryScript := h.ServiceDir("tilebuilder").File("percent-of-available-memory")
	memoryBudget, err := container.
		WithFile("percent-of-available-memory", memoryScript).
		WithExec([]string{"/percent-of-available-memory", "75"}).
		Stdout(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to compute memory budget: %w", err)
	}

	container = container.
		WithExec([]string{"mkdir", "-p", "/data/sources"}).
		WithExec([]string{"sh", "-c", "curl --no-progress-meter https://data.maps.earth/planetiler_fixtures/sources.tar | tar -x --directory /data/sources"}).
		WithMountedFile("/data/data.osm.pbf", h.OSMExport.File)

	entrypoint, err := container.Entrypoint(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get entrypoint: %w", err)
	}

	if h.IsPlanetBuild {
		container = container.WithExec(append(entrypoint,
			"--osm_path=/data/data.osm.pbf",
			"--force",
			"--bounds=planet",
			"--nodemap-type=array",
			"--storage=mmap",
			fmt.Sprintf("-Xmx%d", memoryBudget),
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
	Config  *dagger.File
	Headway *Headway
}

// We use this both for import and for production pelias instances.
// But we might want to try a longer timeout for the import process?
func (h *Headway) Pelias(ctx context.Context) *Pelias {
	countriesStr := h.Countries
	config := slimNodeContainer().
		WithDirectory("generate_config", h.ServiceDir("pelias").Directory("generate_config")).
		WithWorkdir("generate_config").
		WithExec([]string{"yarn", "install"}).
		WithExec([]string{"yarn", "build"}).
		WithExec([]string{"sh", "-c", fmt.Sprintf("bin/generate-pelias-config areas.csv '%s' '%s' > pelias.json", h.Area, countriesStr)}).
		File("pelias.json")

	return &Pelias{Config: config, Headway: h}
}

func (p *Pelias) PeliasContainerFrom(containerName string) *dagger.Container {
	container := dag.Container().
		From(containerName).
		WithMountedDirectory("/pelias-service", p.Headway.ServiceDir("pelias")).
		WithFile("/code/pelias.json", p.Config)
	return container
}

func (p *Pelias) DownloadWhosOnFirst(ctx context.Context) *dagger.Directory {
	container := p.PeliasContainerFrom("pelias/whosonfirst:master").
		WithExec([]string{"./bin/download"})
	return container.Directory("/data/whosonfirst")
}

func (p *Pelias) DownloadOpenAddresses(ctx context.Context) *dagger.Directory {
	container := p.PeliasContainerFrom("pelias/openaddresses:master").
		WithExec([]string{"./bin/download"})
	return container.Directory("/data/openaddresses")
}

func (p *Pelias) PreparePlaceholder(ctx context.Context) *Artifact {
	container := p.PeliasContainerFrom("pelias/placeholder:master").
		WithMountedDirectory("/data/whosonfirst", p.DownloadWhosOnFirst(ctx)).
		WithExec([]string{"bash", "-c", "./cmd/extract.sh && ./cmd/build.sh"})
	return &Artifact{Directory: container.Directory("/data/placeholder")}
}

type PeliasImporter struct {
	Pelias                   *Pelias
	ElasticsearchCacheVolume *dagger.CacheVolume
	ElasticsearchService     *dagger.Service
}

func (p *Pelias) Importer(ctx context.Context) *PeliasImporter {
	cacheKey := time.Now().Format("2006-01")
	elasticsearchCache := dag.CacheVolume(fmt.Sprintf("pelias-elasticsearch-%s-%s", p.Headway.Area, cacheKey))

	// REVIEW: Sharing? Would "PRIVATE" allow us to get rid of the cache buster?
	// maybe cache buster should be based on the input not timestamp
	opts := dagger.ContainerWithMountedCacheOpts{Owner: "elasticsearch", Sharing: "SHARED"}

	// NOTE: docker-compose passes some extra arguments to this container, e.g. IPC and mem size
	elasticsearchService := dag.Container().
		From("pelias/elasticsearch:8.12.2-beta").
		WithExposedPort(9200).
		WithMountedCache("/usr/share/elasticsearch/data", elasticsearchCache, opts).
		AsService()

	return &PeliasImporter{
		Pelias:                   p,
		ElasticsearchCacheVolume: elasticsearchCache,
		ElasticsearchService:     elasticsearchService,
	}
}

func (p *PeliasImporter) ImporterContainerFrom(containerName string) *dagger.Container {
	return p.Pelias.PeliasContainerFrom(containerName).
		WithServiceBinding("pelias-elasticsearch", p.ElasticsearchService).
		WithExec([]string{"/pelias-service/wait.sh"})
}

func (p *PeliasImporter) ImportSchema(ctx context.Context) *dagger.Container {
	return p.ImporterContainerFrom("pelias/schema:master").
		WithExec([]string{"./bin/create_index"})
}

func (p *PeliasImporter) ImportWhosOnFirst(ctx context.Context) *dagger.Container {
	return p.ImporterContainerFrom("pelias/whosonfirst:master").
		WithMountedDirectory("/data/whosonfirst", p.Pelias.DownloadWhosOnFirst(ctx)).
		WithExec([]string{"./bin/start"})
}

func (p *PeliasImporter) ImportOpenAddresses(ctx context.Context) *dagger.Container {
	return p.ImporterContainerFrom("pelias/openaddresses:master").
		WithMountedDirectory("/data/openaddresses", p.Pelias.DownloadOpenAddresses(ctx)).
		// OpenAddress import also uses WhosOnFirst data
		WithMountedDirectory("/data/whosonfirst", p.Pelias.DownloadWhosOnFirst(ctx)).
		WithExec([]string{"./bin/start"})
}

func (p *PeliasImporter) ImportOpenStreetMap(ctx context.Context) *dagger.Container {
	if p.Pelias.Headway.OSMExport == nil || p.Pelias.Headway.OSMExport.File == nil {
		panic("PeliasImporter: Headway.OSMExport.File must be set to import OpenStreetMap data")
	}

	return p.ImporterContainerFrom("pelias/openstreetmap:master").
		WithMountedFile("/data/openstreetmap/data.osm.pbf", p.Pelias.Headway.OSMExport.File).
		// OpenStreetMap import also uses WhosOnFirst data
		WithMountedDirectory("/data/whosonfirst", p.Pelias.DownloadWhosOnFirst(ctx)).
		WithExec([]string{"./bin/start"})
}

func (p *PeliasImporter) ImportPolylines(ctx context.Context) *dagger.Container {
	return p.ImporterContainerFrom("pelias/polylines:master").
		WithMountedFile("/data/polylines/extract.0sv", p.Pelias.Headway.ValhallaPolylines(ctx)).
		// Polylines import also uses WhosOnFirst data
		WithMountedDirectory("/data/whosonfirst", p.Pelias.DownloadWhosOnFirst(ctx)).
		WithExec([]string{"./bin/start"})
}

func (p *Pelias) ElasticsearchData(ctx context.Context) *Artifact {
	err := error(nil)

	importer := p.Importer(ctx)

	_, err = importer.ImportSchema(ctx).
		Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to import pelias schema: %w", err))
	}

	_, err = importer.ImportWhosOnFirst(ctx).
		Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to import WhoseOnFirst data: %w", err))
	}

	_, err = importer.ImportOpenAddresses(ctx).
		Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to import OpenAddresses data: %w", err))
	}

	_, err = importer.ImportOpenStreetMap(ctx).
		Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to import OpenStreetMap data: %w", err))
	}

	_, err = importer.ImportPolylines(ctx).
		Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to import OpenStreetMap data: %w", err))
	}

	opts := dagger.ContainerWithMountedCacheOpts{Owner: "elasticsearch", Sharing: "SHARED"}
	directory := slimContainer().
		// The cache "Owner" is two things:
		//    1. the owner on the filesystem (as in `chown $owner`)
		//    2. a namespace within the cache, so the same cache will contain different data depending on the Owner argument
		//
		// We need the "elasticsearch" cache, but mounting will error if the user doesn't exist, so we add the user
		WithExec([]string{"useradd", "elasticsearch"}).
		WithMountedCache("/data-cache", importer.ElasticsearchCacheVolume, opts).
		WithExec([]string{"cp", "-r", "/data-cache", "/export"}, dagger.ContainerWithExecOpts{UseEntrypoint: false}).
		Directory("/export")
	return &Artifact{Directory: directory}
}

/**
 * Valhalla
 */
func valhallaBaseContainer() *dagger.Container {
	return dag.Container().
		From("ghcr.io/valhalla/valhalla:latest").
		WithExec([]string{"useradd", "-s", "/usr/sbin/nologin", "valhalla"}).
		WithUser("valhalla").
		WithWorkdir("/tiles")
}

func valhallaBuildContainer() *dagger.Container {
	return valhallaBaseContainer().
		WithExec([]string{"sh", "-c", "valhalla_build_config --mjolnir-tile-dir /tiles --mjolnir-timezone /tiles/timezones.sqlite --mjolnir-admin /tiles/admins.sqlite > valhalla.json"}).
		WithExec([]string{"sh", "-c", "valhalla_build_timezones > /tiles/timezones.sqlite"})
}

// Builds Valhalla routing tiles
func (h *Headway) ValhallaTiles(ctx context.Context) *Artifact {
	if h.OSMExport == nil || h.OSMExport.File == nil {
		panic("Headway.OSMExport.File must be set to build Valhalla tiles")
	}

	container := valhallaBuildContainer().
		WithMountedFile("/data/osm/data.osm.pbf", h.OSMExport.File).
		WithExec([]string{"valhalla_build_tiles", "-c", "valhalla.json", "/data/osm/data.osm.pbf"})

	return &Artifact{Directory: container.Directory("/tiles")}
}

func (h *Headway) ValhallaPolylines(ctx context.Context) *dagger.File {
	container := valhallaBuildContainer().
		WithMountedDirectory("/tiles", h.ValhallaTiles(ctx).Directory).
		WithExec([]string{"sh", "-c", "valhalla_export_edges -c valhalla.json > /tiles/polylines.0sv"})

	return container.File("/tiles/polylines.0sv")
}

func (h *Headway) ValhallaInitContainer(ctx context.Context) *dagger.Container {
	container := valhallaBaseContainer().
		WithUser("root")
	container = WithAptPackages(container, "wget", "zstd")
	return container.
		WithFile("/app/init.sh", h.ServiceDir("valhalla").File("init.sh")).
		WithEntrypoint([]string{"/bin/bash"}).
		WithDefaultArgs([]string{"/app/init.sh"})
}

func (h *Headway) ValhallaServeContainer(ctx context.Context) *dagger.Container {
	return valhallaBaseContainer().
		WithEntrypoint([]string{"valhalla_service"}).
		WithDefaultArgs([]string{"/data/valhalla.json"})
}

// Extracts bounding box for a given area from bboxes.csv
func (h *Headway) BBox(ctx context.Context) (*Bbox, error) {
	bboxesFile := h.ServiceDir("gtfs").File("bboxes.csv")

	// Area name to look up (must exist in bboxes.csv)
	area := h.Area
	if area == "" {
		return nil, fmt.Errorf("Area is required to get bounding box")
	}

	container := slimContainer().
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
	container := slimContainer("unzip").
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
	sourceDir := h.ServiceDir("gtfs/gtfout")
	container := rustContainer().
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
	return elevations(ctx, bbox, t.Headway)
}

func (h *Headway) Elevations(ctx context.Context) *dagger.Directory {
	bbox, err := h.BBox(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to get bounding box: %w", err))
	}
	return elevations(ctx, bbox, h)
}

func elevations(ctx context.Context, bbox *Bbox, headway *Headway) *dagger.Directory {
	elevationHgts := valhallaBaseContainer().
		WithExec([]string{"valhalla_build_elevation", "--outdir", "elevation-hgts", "--from-bbox=" + bbox.CommaSeparated()}).
		Directory("/tiles/elevation-hgts")

	// Convert elevation HGT files to TIF format
	demScript := headway.ServiceDir("otp").File("dem-hgt-to-tif")
	container := slimContainer("gdal-bin").
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

// ===
// Travelmux
// ===

func (h *Headway) TravelmuxServer(ctx context.Context) *dagger.File {
	return rustContainer().
		WithWorkdir("app").
		// This speeds up rebuilds of rust projects by caching the prebuilt
		// dependencies in a separate docker layer. Without this, every change to
		// the source requires re-downloading and re-building all the project deps,
		// which takes a while.
		// WithMountedFile("Cargo.toml", h.ServicesDir.Directory("travelmux").File("Cargo.toml")).
		// WithMountedFile("Cargo.lock", h.ServicesDir.Directory("travelmux").File("Cargo.lock")).
		// WithExec([]string{"mkdir", "-p", "src"}).
		// WithExec([]string{"sh", "-c", "echo 'fn main() { /* dummy main to get cargo to build deps */ }' > src/main.rs"}).
		// WithExec([]string{"cargo", "build", "--release"}).
		// WithExec([]string{"rm", "src/main.rs"}).
		WithMountedDirectory("/app", h.ServiceDir("travelmux")).
		WithExec([]string{"cargo", "build", "--release"}).
		File("target/release/travelmux-server")
}

func (h *Headway) TravelmuxServeContainer(ctx context.Context) *dagger.Container {
	serverBin := h.TravelmuxServer(ctx)

	container := slimContainer("libssl3").
		WithExec([]string{"adduser", "--disabled-login", "travelmux", "--gecos", ""}).
		WithUser("travelmux").
		WithWorkdir("/home/travelmux").
		WithFile("/home/travelmux/travelmux-server", serverBin, dagger.ContainerWithFileOpts{Permissions: 0755}).
		WithExposedPort(8000).
		WithEnvVariable("RUST_LOG", "info").
		WithEntrypoint([]string{"/home/travelmux/travelmux-server"}).
		WithDefaultArgs([]string{"http://valhalla:8002", "http://opentripplanner:8000/otp/routers"})

	return container
}

func (h *Headway) TravelmuxInitContainer(ctx context.Context) *dagger.Container {
	return downloadContainer().
		WithFile("/app/init.sh", h.ServiceDir("travelmux").File("init.sh"), dagger.ContainerWithFileOpts{Permissions: 0755}).
		WithDefaultArgs([]string{"/app/init.sh"})
}

/**
 * Web Frontend
 */

func (h *Headway) WebBuild(ctx context.Context,
	// +optional
	branding string) *dagger.Directory {
	container := slimNodeContainer().
		WithExec([]string{"yarn", "global", "add", "@quasar/cli"}).
		WithMountedDirectory("/www-app", h.ServiceDir("frontend/www-app")).
		WithWorkdir("/www-app")

	if branding != "" {
		container = container.WithExec([]string{"sed", "-i", "s/.*productName.*/  \"productName\": \"" + branding + "\",/", "package.json"})
	}

	return container.
		WithExec([]string{"yarn", "install"}).
		WithExec([]string{"quasar", "build"}).
		Directory("/www-app/dist/spa")
}

func (h *Headway) WebServeContainer(ctx context.Context,
	// +optional
	branding string) *dagger.Container {
	webBuild := h.WebBuild(ctx, branding)

	return dag.Container().
		From("nginx").
		WithDirectory("/usr/share/nginx/html/", webBuild).
		WithFile("/etc/nginx/templates/nginx.conf.template", h.ServiceDir("frontend").File("nginx.conf.template")).
		WithEnvVariable("HEADWAY_PUBLIC_URL", "http://127.0.0.1:8080").
		WithEnvVariable("HEADWAY_SHARED_VOL", "/data").
		WithEnvVariable("HEADWAY_HTTP_PORT", "8080").
		WithEnvVariable("HEADWAY_RESOLVER", "127.0.0.11").
		WithEnvVariable("HEADWAY_TRAVELMUX_URL", "http://travelmux:8000").
		WithEnvVariable("HEADWAY_TILESERVER_URL", "http://tileserver:8000").
		WithEnvVariable("HEADWAY_PELIAS_URL", "http://pelias-api:8080").
		WithEnvVariable("ESC", "$").
		WithEnvVariable("NGINX_ENVSUBST_OUTPUT_DIR", "/etc/nginx")
}

func (h *Headway) WebInitContainer(ctx context.Context) *dagger.Container {
	return downloadContainer().
		WithFile("/app/init.sh", h.ServiceDir("frontend").File("init.sh")).
		WithFile("/app/generate_config.sh", h.ServiceDir("frontend").File("generate_config.sh")).
		WithEnvVariable("HEADWAY_SHARED_VOL", "/data").
		WithDefaultArgs([]string{"/app/init.sh"})
}

/**
 * Pelias
 */

func (h *Headway) PeliasInitContainer(ctx context.Context) *dagger.Container {
	return downloadContainer().
		WithExec([]string{"mkdir", "-p", "/app"}).
		WithFile("/app/", h.ServiceDir("pelias").File("init_config.sh")).
		WithFile("/app/", h.ServiceDir("pelias").File("init_elastic.sh")).
		WithFile("/app/", h.ServiceDir("pelias").File("init_placeholder.sh")).
		WithDefaultArgs([]string{"echo", "run a specific command"})
}

// ===
// Helpers
// ===

func slimContainer(packages ...string) *dagger.Container {
	container := dag.Container().From("debian:bookworm-slim")
	if len(packages) == 0 {
		return container
	}
	return WithAptPackages(container, packages...)
}

func rustContainer(packages ...string) *dagger.Container {
	container := dag.Container().From("rust:bookworm")
	if len(packages) == 0 {
		return container
	}
	return WithAptPackages(container, packages...)
}

func slimNodeContainer(packages ...string) *dagger.Container {
	container := dag.Container().From("node:20-slim")
	if len(packages) == 0 {
		return container
	}
	return WithAptPackages(container, packages...)
}

func downloadContainer() *dagger.Container {
	return slimContainer("wget", "ca-certificates", "zstd").
		WithWorkdir("/data")
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
	container := slimContainer("zstd").
		WithExec([]string{"mkdir", "/app"}).
		WithWorkdir("/app").
		WithMountedDirectory("/app/input", dir).
		WithExec([]string{"sh", "-c", "tar --use-compress-program='zstd -T0' -cf output.tar.zst -C input ."})

	return container.File("output.tar.zst")
}

func compressFile(input *dagger.File) *dagger.File {
	container := slimContainer("zstd").
		WithExec([]string{"mkdir", "/app"}).
		WithWorkdir("/app").
		WithMountedFile("/app/input", input).
		WithExec([]string{"sh", "-c", "zstd -T0 input"})

	return container.File("input.zst")
}
