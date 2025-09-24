// Headway mapping stack build system
//
// This Dagger module builds the Headway mapping stack, which consists of:
//
// - Map tiles for the tileserver
// - Routing graphs for Valhalla (car/bike/walking) and OpenTripPlanner (transit)
// - Pelias geocoding index for address search
// - Container images for all services (web UI, APIs, databases)
//
// The build system processes OpenStreetMap data, GTFS transit feeds, and other
// geographic data sources to create a complete mapping solution for a given area.

package main

import (
	"context"
	"dagger/headway/internal/dagger"
	"fmt"
	"strconv"
	"strings"
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
func (b *Bbox) SpaceSeparated() string {
	return fmt.Sprintf("%f %f %f %f", b.Left, b.Bottom, b.Right, b.Top)
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

func New(
	// +defaultPath="./services"
	servicesDir *dagger.Directory) *Headway {
	return &Headway{ServicesDir: servicesDir}
}

func (h *Headway) WithArea(
	// Area name (e.g. "Seattle")
	area string,

	// +optional
	countries string,

	// Local OSM PBF file to mount, if missing will download from bbike based on area name
	// +defaultPath=""
	localPbf *dagger.File,
) *Headway {
	h.IsPlanetBuild = countries == "ALL"
	h.Countries = countries
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

	output = output.WithFile(h.Area+".osm.pbf", h.OSMExport.File)

	mbtiles, err := h.Mbtiles(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to build mbtiles: %w", err)
	}
	output = output.WithFile(h.Area+".mbtiles", mbtiles)

	valhalla := h.ValhallaTiles(ctx)
	output = output.WithFile(h.Area+".valhalla.tar.zst", valhalla.Compress())

	pelias := h.Pelias(ctx)
	output = output.WithFile(h.Area+".pelias.json", pelias.Config)

	elasticSearch := pelias.ElasticsearchData(ctx)
	output = output.WithFile(h.Area+".elasticsearch.tar.zst", elasticSearch.Compress())

	placeholder := pelias.PreparePlaceholder(ctx)
	output = output.WithFile(h.Area+".placeholder.tar.zst", placeholder.Compress())

	terrain, err := h.TileserverTerrain(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to download tileserver terrain: %w", err)
	}
	output = output.WithFile("terrain.mbtiles", terrain.File("terrain.mbtiles"))
	output = output.WithFile("landcover.mbtiles", terrain.File("landcover.mbtiles"))

	return output, nil
}

func (o *OSMExport) Clip(ctx context.Context, bbox *Bbox) *OSMExport {
	container := slimContainer("osmium-tool").
		WithExec([]string{"mkdir", "-p", "/app"}).
		WithMountedFile("/app/data.osm.pbf", o.File).
		WithExec([]string{"osmium", "extract", "--bbox", bbox.CommaSeparated(), "--output", "/app/clipped.osm.pbf", "/app/data.osm.pbf"})

	return &OSMExport{File: container.File("/app/clipped.osm.pbf")}
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
		WithEnvVariable("HEADWAY_VALHALLA_URL", "http://valhalla:8002").
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
	container := dag.Container().From("node:22-slim")
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
