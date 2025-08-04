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

type Headway struct{}

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
        err := container.ExportImage(ctx, "gghcr.io/headwaymaps/" + imageName + ":" + tag)
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

// Returns a container with the specified apt packages installed
func WithAptPackages(container *dagger.Container, packages ...string) *dagger.Container {
    if len(packages) == 0 {
        return container
    }

    pkgList := strings.Join(packages, " ")
    cmd := fmt.Sprintf("apt-get update && apt-get install -y --no-install-recommends %s && rm -rf /var/lib/apt/lists/*", pkgList)

    return container.WithExec([]string{"sh", "-c", cmd})
}

