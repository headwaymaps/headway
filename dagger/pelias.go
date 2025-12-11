package main

import (
	"context"
	"dagger/headway/internal/dagger"
	"fmt"
	"strings"
)

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
		WithFile("areas.csv", h.ServicesDir.File("areas.csv")).
		WithExec([]string{"yarn", "install", "--frozen-lockfile", "--ignore-scripts"}).
		WithExec([]string{"yarn", "build"}).
		WithExec([]string{"sh", "-c", fmt.Sprintf("bin/generate-pelias-config areas.csv '%s' '%s' > pelias.json", h.Area, countriesStr)}).
		// Strip devDependencies from final image
		WithExec([]string{"yarn", "install", "--prod", "--frozen-lockfile", "--ignore-scripts"}).
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

func (p *Pelias) OpenAddressesIsEnabled(ctx context.Context) bool {
	if p.Headway.IsPlanetBuild {
		return true
	}
	configStr, err := p.Config.Contents(ctx)
	if err != nil {
		panic("unable to read pelias config")
	}
	// This is a crude check, but good enough for now
	return strings.Contains(configStr, "openaddresses")
}

type PeliasImporter struct {
	Pelias                   *Pelias
	ElasticsearchCacheVolume *dagger.CacheVolume
	ElasticsearchService     *dagger.Service
}

func (p *Pelias) Importer(ctx context.Context) *PeliasImporter {
	osmPbfDigest, err := p.Headway.OSMExport.File.Digest(ctx)
	if err != nil {
		panic("unable to digest OSM PBF")
	}
	cacheKey := fmt.Sprintf("3-content-%s", osmPbfDigest)
	elasticsearchCache := dag.CacheVolume(fmt.Sprintf("pelias-elasticsearch-%s-%s", p.Headway.Area, cacheKey))

	opts := dagger.ContainerWithMountedCacheOpts{Owner: "elasticsearch", Sharing: "SHARED"}

	// NOTE: docker compose passes some extra arguments to this container, e.g. IPC and mem size
	elasticsearchService := dag.Container().
		From("pelias/elasticsearch:8.12.2-beta").
		WithEnvVariable("ES_JAVA_OPTS", "-Xmx8g").
		// ulimits:
		//    memlock:
		//      soft: -1
		//      hard: -1
		//    nofile:
		//      soft: 65536
		//      hard: 65536
		// cap_add: [ "IPC_LOCK" ]
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
		WithExec([]string{"npm", "run", "parallel", "3"})
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

	_, err = importer.ElasticsearchService.Start(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to start elasticsearch service: %w", err))
	}

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

	if p.OpenAddressesIsEnabled(ctx) {
		_, err = importer.ImportOpenAddresses(ctx).
			Sync(ctx)
		if err != nil {
			panic(fmt.Errorf("failed to import OpenAddresses data: %w", err))
		}
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

	_, err = importer.ElasticsearchService.Stop(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to stop elasticsearch service: %w", err))
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

func (h *Headway) PeliasInitContainer(ctx context.Context) *dagger.Container {
	return downloadContainer().
		WithExec([]string{"mkdir", "-p", "/app"}).
		WithFile("/app/", h.ServiceDir("pelias").File("init_config.sh")).
		WithFile("/app/", h.ServiceDir("pelias").File("init_elastic.sh")).
		WithFile("/app/", h.ServiceDir("pelias").File("init_placeholder.sh")).
		WithDefaultArgs([]string{"echo", "run a specific command"})
}
