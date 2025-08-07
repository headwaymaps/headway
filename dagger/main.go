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
)

type Headway struct {
}
type Bbox struct {
	Value string
}

// Downloads terrain tiles from headway-data repository
func (m *Headway) TileserverTerrain(ctx context.Context) (*dagger.Directory, error) {
	assetRoot := "https://github.com/headwaymaps/headway-data/raw/main/tiles/"

	container := downloadContainer().
		WithExec([]string{"wget", "-nv", assetRoot + "terrain.mbtiles"}).
		WithExec([]string{"wget", "-nv", assetRoot + "landcover.mbtiles"})

	return container.Directory("/data"), nil
}

// Build assets for the tileserver
func (m *Headway) TileserverAssets(ctx context.Context,
	// +defaultPath="./services/tileserver/assets"
	assetsDir *dagger.Directory) (*dagger.Directory, error) {
	container := dag.Container().
		From("rust:bookworm")

	container = WithAptPackages(container, "libfreetype6-dev").
		WithMountedDirectory("/app/assets", assetsDir).
		WithExec([]string{"cargo", "install", "spreet", "build_pbf_glyphs"}).

		// FONTS
		WithWorkdir("/app/assets/fonts").
		WithExec([]string{"build_pbf_glyphs", "./", "/output/fonts"}).

		// SPRITES
		WithExec([]string{"mkdir", "-p", "/output/sprites"}).
		WithWorkdir("/app/assets/sprites").
		WithExec([]string{"spreet", "./", "/output/sprites/sprite"}).
		WithExec([]string{"spreet", "--retina", "./", "/output/sprites/sprite@2x"})

	return container.Directory("/output"), nil
}

// Build tileserver init container image
func (m *Headway) TileserverInitImage(ctx context.Context,
	// +defaultPath="./services/tileserver"
	serviceDir *dagger.Directory,
) *dagger.Container {
	return downloadContainer().
		WithFile("/app/init.sh", serviceDir.File("init.sh")).
		WithDefaultArgs([]string{"/app/init.sh"})
}

// FIXME: ExportImage doesn't work.  (but dagger shell: export-image does work!)
func (m *Headway) ExportTileserverInitImage(ctx context.Context,
	// +defaultPath="./services/tileserver"
	serviceDir *dagger.Directory,
	tags []string,
) error {
	container := m.TileserverInitImage(ctx, serviceDir)
	return m.ExportContainerImage(ctx, container, "tileserver-init", tags)
}

/**
* Helpers
 */

// Export the given container
func (m *Headway) ExportContainerImage(ctx context.Context,
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

// Generates today's date for cache busting purposes
func (m *Headway) CacheBuster(ctx context.Context) (string, error) {
	container := dag.Container().
		From("debian:bookworm-slim").
		WithExec([]string{"date", "+%Y-%m-%d"})

	return container.Stdout(ctx)
}

// Extracts bounding box for a given area from bboxes.csv
func (m *Headway) Bbox(ctx context.Context,
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
func (m *Headway) GtfsGetMobilitydb(ctx context.Context) *dagger.File {
	container := downloadContainer().
		WithExec([]string{"wget", "-O", "mobilitydb.csv", "https://storage.googleapis.com/storage/v1/b/mdb-csv/o/sources.csv?alt=media"})

	return container.File("/data/mobilitydb.csv")
}

// Builds Rust GTFS processing tools
// I'm not yet sure how exporting will work in situ. Something akin to:
//
//	dagger -c 'gtfout | file assume-bikes-allowed | export ./assume-bikes-allowed'
func (m *Headway) Gtfout(ctx context.Context,
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
		WithExec([]string{"chmod", "+x", "/dem-hgt-to-tif"}).
		WithMountedDirectory("/elevation-hgts", elevationHgts).
		WithExec([]string{"/dem-hgt-to-tif", "/elevation-hgts", "/elevation-tifs"})

	return container.Directory("/elevation-tifs")
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
