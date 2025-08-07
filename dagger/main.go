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
	"strings"
	"time"
)

type Headway struct {
	OSMExport *OSMExport
}

type Bbox struct {
	Value string
}

type OSMExport struct {
	// PBF file
	File *dagger.File
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

func (h *Headway) TestDind(ctx context.Context) string {
	tmp := dag.CacheVolume("temp")
	ctr := dag.
		Container().
		From("index.docker.io/docker:24.0-dind").
		WithMountedCache("/temp", tmp).
		WithMountedCache("/var/lib/docker", dag.CacheVolume("dind-lib-docker")).
		WithoutEntrypoint().
		WithExposedPort(2375)

	out, _ := dag.Container().From("docker:cli").
		WithServiceBinding("docker", ctr.AsService(dagger.ContainerAsServiceOpts{
			Args: []string{
				"dockerd",
				"--host=tcp://0.0.0.0:2375",
				"--host=unix:///var/run/docker.sock",
				"--tls=false",
			},
			InsecureRootCapabilities: true,
		})).
		WithEnvVariable("DOCKER_HOST", "tcp://docker:2375").
		WithMountedCache("/temp", tmp).
		WithNewFile("foo.txt", "Hello").
		WithExec([]string{"cp", "foo.txt", "/temp/foo.txt"}).
		WithExec([]string{"docker", "run", "-v", "/temp/foo.txt:/foo.txt", "alpine", "cat", "/foo.txt"}).
		Stdout(ctx)
	return out
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

type PeliasImporter struct {
	Pelias               *Pelias
	ElasticsearchCache   *dagger.CacheVolume
	ElasticsearchService *dagger.Service
}

/*
func (p *Pelias) PeliasPreparePlaceholder(ctx context.Context) *Pelias {
	p = p.PeliasWhosOnFirst(ctx)
	container := p.PeliasContainerFrom(ctx, "pelias/placeholder:master").
		WithExec([]string{"bash", "-c", "./cmd/extract.sh && ./cmd/build.sh"})
	p.DataDir = container.Directory("/data")
	container.Terminal()
	return p
}
*/

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
	return p.PeliasImporterContainerFrom("pelias/openstreetmap:master").
		WithMountedFile("/data/openstreetmap/data.osm.pbf", p.Pelias.Headway.OSMExport.File).
		// OpenStreetMap import also uses WhosOnFirst data
		WithMountedDirectory("/data/whosonfirst", p.Pelias.PeliasDownloadWhosOnFirst()).
		WithExec([]string{"./bin/start"})
}

func (p *PeliasImporter) PeliasImportPolylines() *dagger.Container {
	return p.PeliasImporterContainerFrom("pelias/polylines:master").
		WithMountedFile("/data/polylines/extract.0sv", p.Pelias.Headway.OSMExport.ValhallaPolylines()).
		// Polylines import also uses WhosOnFirst data
		WithMountedDirectory("/data/whosonfirst", p.Pelias.PeliasDownloadWhosOnFirst()).
		WithExec([]string{"./bin/start"})
}

func (p *Pelias) PeliasImport(ctx context.Context) *dagger.Directory {
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
 * Helpers
 */

// Export the given container
func (h *Headway) ExportContainerImage(ctx context.Context,
	container *dagger.Container,
	imageName string,
	tags []string,
) error {
	for _, tag := range tags {
		err := container.ExportImage(ctx, "gghcr.io/headwaymaps/"+imageName+":"+tag)
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

func valhallaBaseContainer() *dagger.Container {
	return dag.Container().
		From("ghcr.io/gis-ops/docker-valhalla/valhalla").
		WithUser("root").
		WithWorkdir("/tiles").
		WithExec([]string{"chown", "valhalla", "/tiles"}).
		WithUser("valhalla")
}

func valhallaBuildContainer() *dagger.Container {
	return valhallaBaseContainer().
		WithExec([]string{"sh", "-c", "valhalla_build_config --mjolnir-tile-dir /tiles --mjolnir-timezone /tiles/timezones.sqlite --mjolnir-admin /tiles/admins.sqlite > valhalla.json"}).
		WithExec([]string{"sh", "-c", "valhalla_build_timezones > /tiles/timezones.sqlite"})
}

// Extracts bounding box for a given area from bboxes.csv
func (h *Headway) BBox(ctx context.Context,
	// Area name to look up (must exist in bboxes.csv)
	area string,
	// +defaultPath="./services/gtfs/bboxes.csv"
	bboxesFile *dagger.File) (Bbox, error) {

	container := dag.Container().
		From("debian:bookworm-slim").
		WithMountedFile("/bboxes.csv", bboxesFile).
		WithExec([]string{"sh", "-c", fmt.Sprintf("test $(grep '%s:' /bboxes.csv | wc -l) -eq 1", area)}).
		WithExec([]string{"sh", "-c", fmt.Sprintf("grep '%s:' /bboxes.csv | cut -d':' -f2", area)})

	bboxStr, err := container.Stdout(ctx)
	if err != nil {
		return Bbox{}, fmt.Errorf("failed to get bbox for area %s: %w", area, err)
	}
	return Bbox{Value: bboxStr}, nil
}

// Downloads GTFS mobility database CSV
func (h *Headway) GtfsGetMobilitydb(ctx context.Context) *dagger.File {
	container := downloadContainer().
		WithExec([]string{"wget", "-O", "mobilitydb.csv", "https://storage.googleapis.com/storage/v1/b/mdb-csv/o/sources.csv?alt=media"})

	return container.File("/data/mobilitydb.csv")
}

// Builds Rust GTFS processing tools
// I'm not yet sure how exporting will work in situ. Something akin to:
//
//	dagger -c 'gtfout | file assume-bikes-allowed | export ./assume-bikes-allowed'
func (h *Headway) Gtfout(ctx context.Context,
	// +defaultPath="./services/gtfs/gtfout"
	sourceDir *dagger.Directory) *dagger.Directory {

	container := dag.Container().
		From("rust:bookworm").
		WithMountedDirectory("/gtfout", sourceDir).
		WithWorkdir("/gtfout").
		WithExec([]string{"cargo", "build", "--release"})

	return container.Directory("/gtfout/target/release")
}

// Downloads elevation data for a given bounding box
func (m *Bbox) DownloadElevation(ctx context.Context) *dagger.Directory {

	// Convert space-separated bbox to comma-separated format for valhalla
	valhallaBbox := strings.ReplaceAll(m.Value, " ", ",")

	container := valhallaBaseContainer().
		WithExec([]string{"valhalla_build_elevation", "--outdir", "elevation-hgts", "--from-bbox=" + valhallaBbox})

	return container.Directory("/tiles/elevation-hgts")
}

// Converts elevation HGT files to TIF format
func (m *Bbox) Elevation(ctx context.Context,
	// +defaultPath="./services/otp/dem-hgt-to-tif"
	demScript *dagger.File) *dagger.Directory {

	elevationHgts := m.DownloadElevation(ctx)

	container := dag.Container().
		From("debian:bookworm-slim")

	container = WithAptPackages(container, "gdal-bin").
		WithMountedFile("/dem-hgt-to-tif", demScript).
		WithMountedDirectory("/elevation-hgts", elevationHgts).
		WithExec([]string{"/dem-hgt-to-tif", "/elevation-hgts", "/elevation-tifs"})

	return container.Directory("/elevation-tifs")
}

// Mounts a local OSM PBF file
func (h *Headway) DownloadPBF(
	// Area name (e.g. "Seattle")
	area string) *OSMExport {

	downloadUrl := fmt.Sprintf("https://download.bbbike.org/osm/bbbike/%s/%s.osm.pbf", area, area)
	container := downloadContainer().WithExec([]string{"wget", "-nv", "-U", "headway/1.0", "-O", "data.osm.pbf", downloadUrl})

	return &OSMExport{File: container.File("/data/data.osm.pbf")}
}

// Mounts a local OSM PBF file
func (h *Headway) LocalPBF(
	// Local OSM PBF file to mount
	pbfFile *dagger.File) *OSMExport {
	return &OSMExport{
		File: pbfFile,
	}
}

// Downloads OSM extract from bbike
func (h *Headway) WithDownloadedPBF(
	// Area name (e.g. "Seattle")
	area string) *Headway {

	h.OSMExport = h.DownloadPBF(area)
	return h
}

// Mounts a local OSM PBF file
func (h *Headway) WithLocalPBF(
	// Local OSM PBF file to mount
	pbfFile *dagger.File) *Headway {

	h.OSMExport = h.LocalPBF(pbfFile)
	return h
}

// Builds mbtiles using Planetiler
func (m *OSMExport) Mbtiles(ctx context.Context,
	// +optional
	// Whether this is a planet-scale build (affects memory settings)
	isPlanetBuild bool,
	// +defaultPath="./services/tilebuilder/percent-of-available-memory"
	memoryScript *dagger.File) (*dagger.File, error) {

	container := dag.Container().
		From("ghcr.io/onthegomap/planetiler:0.7.0").
		WithExec([]string{"mkdir", "-p", "/data/sources"}).
		WithExec([]string{"sh", "-c", "curl --no-progress-meter https://data.maps.earth/planetiler_fixtures/sources.tar | tar -x --directory /data/sources"}).
		WithMountedFile("/data/data.osm.pbf", m.File)

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

// Builds Valhalla routing tiles
func (m *OSMExport) ValhallaTiles() *dagger.Directory {
	container := valhallaBuildContainer().
		WithMountedFile("/data/osm/data.osm.pbf", m.File).
		WithExec([]string{"valhalla_build_tiles", "-c", "valhalla.json", "/data/osm/data.osm.pbf"})

	return container.Directory("/tiles")
}

func (m *OSMExport) ValhallaPolylines() *dagger.File {
	container := valhallaBuildContainer().
		// TODO: probably I need to mount the tiles?
		WithMountedDirectory("/tiles", m.ValhallaTiles()).
		WithExec([]string{"sh", "-c", "valhalla_export_edges -c valhalla.json > /tiles/polylines.0sv"})

	return container.File("/tiles/polylines.0sv")
}

// Returns a container with the specified apt packages installed
func WithAptPackages(container *dagger.Container, packages ...string) *dagger.Container {
	if len(packages) == 0 {
		return container
	}

	pkgList := strings.Join(packages, " ")
	cmd := fmt.Sprintf("apt-get update && apt-get install -y --no-install-recommends %s && rm -rf /var/lib/apt/lists/*", pkgList)

	return container.WithExec([]string{"sh", "-c", cmd})
}
