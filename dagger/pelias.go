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

const (
	peliasElasticsearchImage = "pelias/elasticsearch:8.12.2-beta"
	peliasPlaceholderImage   = "pelias/placeholder:master"
	peliasSchemaImage        = "pelias/schema:master"
	peliasWhosOnFirstImage   = "pelias/whosonfirst:master"
	peliasOpenAddressesImage = "pelias/openaddresses:master"
	peliasOpenStreetMapImage = "pelias/openstreetmap:master"
	peliasPolylinesImage     = "pelias/polylines:master"
)

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
		WithFile("areas.csv", h.ServicesDir().File("areas.csv")).
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
	container := p.PeliasContainerFrom(peliasWhosOnFirstImage).
		WithExec([]string{"./bin/download"})
	return container.Directory("/data/whosonfirst")
}

func (p *Pelias) DownloadOpenAddresses(ctx context.Context) *dagger.Directory {
	container := p.PeliasContainerFrom(peliasOpenAddressesImage).
		WithExec([]string{"./bin/download"})
	return container.Directory("/data/openaddresses")
}

func (p *Pelias) PreparePlaceholder(ctx context.Context) *Artifact {
	container := p.PeliasContainerFrom(peliasPlaceholderImage).
		WithMountedDirectory("/data/whosonfirst", p.DownloadWhosOnFirst(ctx)).
		WithExec([]string{"bash", "-c", "./cmd/extract.sh && ./cmd/build.sh"})
	return DirectoryArtifact(p.Headway.Area+"-placeholder", container.Directory("/data/placeholder"))
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

	// The head of the import chain: the step that empties the elasticsearch
	// data directory. Every import step descends from it.
	reset *dagger.Container
}

// The elasticsearch data lives in a cache volume, which is mutable state that
// dagger's cache keys can't see. That leaves two caches free to disagree: when
// an import step's key changes but the volume still holds the last run's data,
// the step runs against dirty state - "index [pelias] already exists" if we're
// lucky, silently duplicated documents if we're not. Worse, the export at the
// end reads the volume too, so it can hand back data from a run that no longer
// corresponds to its inputs.
//
// Two things keep the caches in lockstep:
//
//  1. The chain starts by emptying the volume. That step mounts every input to
//     the import, so any change to any of them re-runs it.
//  2. Each step carries a marker file from the step before it, making the chain
//     a real dependency in dagger's graph rather than just Go control flow. The
//     reset writes a fresh marker whenever it runs, so re-running it re-runs
//     everything downstream, export included.
//
// The upshot is what you'd want from a build step: the import is cached whole,
// or it runs whole.
const peliasMarkerPath = "/headway-pelias-import-marker"

// importer prepares elasticsearch and the import chain that populates it.
//
// This does real work rather than just describing it: the data directory has to
// be emptied before elasticsearch opens it.
func (p *Pelias) importer(ctx context.Context) *PeliasImporter {
	// The volume name is stable per area. It doesn't need to encode any input
	// digests - it's scratch space, not a cache, and the reset step below is
	// what ties its contents to the inputs.
	elasticsearchCache := dag.CacheVolume(fmt.Sprintf("pelias-elasticsearch-%s", p.Headway.Area))

	importer := &PeliasImporter{
		Pelias:                   p,
		ElasticsearchCacheVolume: elasticsearchCache,
	}

	reset, err := importer.resetElasticsearchData(ctx).Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to reset elasticsearch data: %w", err))
	}
	importer.reset = reset

	marker, err := reset.File(peliasMarkerPath).Contents(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to read pelias import marker: %w", err))
	}

	// NOTE: docker compose passes some extra arguments to this container, e.g. IPC and mem size
	importer.ElasticsearchService = dag.Container().
		From(peliasElasticsearchImage).
		WithEnvVariable("ES_JAVA_OPTS", "-Xmx8g").
		// A new marker means the data directory was just emptied. Stamping it
		// here keeps dagger from handing back a service instance that's still
		// holding the old data open.
		WithEnvVariable("HEADWAY_IMPORT_MARKER", marker).
		// ulimits:
		//    memlock:
		//      soft: -1
		//      hard: -1
		//    nofile:
		//      soft: 65536
		//      hard: 65536
		// cap_add: [ "IPC_LOCK" ]
		WithExposedPort(9200).
		WithMountedCache("/usr/share/elasticsearch/data", elasticsearchCache, peliasCacheOpts()).
		AsService()

	return importer
}

// The cache "Owner" is two things:
//
//  1. the owner on the filesystem (as in `chown $owner`)
//  2. a namespace within the cache, so the same cache will contain different
//     data depending on the Owner argument
const elasticsearchCacheOwner = "elasticsearch"

func peliasCacheOpts() dagger.ContainerWithMountedCacheOpts {
	return dagger.ContainerWithMountedCacheOpts{Owner: elasticsearchCacheOwner, Sharing: "SHARED"}
}

// Mounting the cache errors if its owner doesn't exist, so any image that isn't
// already elasticsearch's needs the user added - under the same name, or it
// lands in a different namespace within the cache and finds it empty.
func withElasticsearchCacheOwner(container *dagger.Container) *dagger.Container {
	return container.WithExec([]string{"useradd", elasticsearchCacheOwner})
}

// resetElasticsearchData empties the elasticsearch data directory and writes a
// fresh marker file for the import chain to carry.
//
// Every input to the import is mounted here, whether or not this step reads it,
// so that dagger's cache key for the reset covers all of them.
func (p *PeliasImporter) resetElasticsearchData(ctx context.Context) *dagger.Container {
	headway := p.Pelias.Headway
	if headway.OSMExport == nil || headway.OSMExport.File == nil {
		panic("PeliasImporter: Headway.OSMExport.File must be set to import OpenStreetMap data")
	}

	container := withElasticsearchCacheOwner(slimContainer()).
		WithMountedFile("/inputs/pelias.json", p.Pelias.Config).
		WithMountedDirectory("/inputs/pelias-service", headway.ServiceDir("pelias")).
		WithMountedFile("/inputs/data.osm.pbf", headway.OSMExport.File).
		WithMountedFile("/inputs/polylines.0sv", headway.ValhallaPolylines(ctx)).
		WithMountedDirectory("/inputs/whosonfirst", p.Pelias.DownloadWhosOnFirst(ctx))

	if p.Pelias.OpenAddressesIsEnabled(ctx) {
		container = container.WithMountedDirectory("/inputs/openaddresses", p.Pelias.DownloadOpenAddresses(ctx))
	}

	// The importer images are floating tags. When one of them moves we want a
	// full reimport, not a second import layered onto the previous one.
	container = container.WithEnvVariable("HEADWAY_PELIAS_IMAGES", p.importerImageRefs(ctx))

	return container.
		WithMountedCache("/data-cache", p.ElasticsearchCacheVolume, peliasCacheOpts()).
		WithExec([]string{"sh", "-c", fmt.Sprintf(
			"rm -rf /data-cache/* /data-cache/.[!.]*; head -c16 /dev/urandom | od -An -tx1 | tr -d ' \\n' > %s",
			peliasMarkerPath,
		)}, dagger.ContainerWithExecOpts{UseEntrypoint: false})
}

// importerImageRefs resolves each importer image to a ref including its digest,
// so a moved tag invalidates the reset step.
func (p *PeliasImporter) importerImageRefs(ctx context.Context) string {
	images := []string{
		peliasElasticsearchImage,
		peliasSchemaImage,
		peliasWhosOnFirstImage,
		peliasOpenAddressesImage,
		peliasOpenStreetMapImage,
		peliasPolylinesImage,
	}

	refs := make([]string, 0, len(images))
	for _, image := range images {
		ref, err := dag.Container().From(image).ImageRef(ctx)
		if err != nil {
			panic(fmt.Errorf("failed to resolve image %q: %w", image, err))
		}
		refs = append(refs, ref)
	}
	return strings.Join(refs, " ")
}

// importerContainerFrom builds an import step that runs after prev.
//
// The marker file it carries over from prev is what makes the ordering visible
// to dagger: if prev re-runs, so does this step.
func (p *PeliasImporter) importerContainerFrom(prev *dagger.Container, containerName string) *dagger.Container {
	return p.Pelias.PeliasContainerFrom(containerName).
		WithServiceBinding("pelias-elasticsearch", p.ElasticsearchService).
		WithFile(peliasMarkerPath, prev.File(peliasMarkerPath)).
		WithExec([]string{"/pelias-service/wait.sh"})
}

func (p *PeliasImporter) importSchema(ctx context.Context, prev *dagger.Container) *dagger.Container {
	return p.importerContainerFrom(prev, peliasSchemaImage).
		WithExec([]string{"./bin/create_index"})
}

func (p *PeliasImporter) importWhosOnFirst(ctx context.Context, prev *dagger.Container) *dagger.Container {
	return p.importerContainerFrom(prev, peliasWhosOnFirstImage).
		WithMountedDirectory("/data/whosonfirst", p.Pelias.DownloadWhosOnFirst(ctx)).
		WithExec([]string{"./bin/start"})
}

func (p *PeliasImporter) importOpenAddresses(ctx context.Context, prev *dagger.Container) *dagger.Container {
	return p.importerContainerFrom(prev, peliasOpenAddressesImage).
		WithMountedDirectory("/data/openaddresses", p.Pelias.DownloadOpenAddresses(ctx)).
		// OpenAddress import also uses WhosOnFirst data
		WithMountedDirectory("/data/whosonfirst", p.Pelias.DownloadWhosOnFirst(ctx)).
		WithExec([]string{"npm", "run", "parallel", "3"})
}

func (p *PeliasImporter) importOpenStreetMap(ctx context.Context, prev *dagger.Container) *dagger.Container {
	return p.importerContainerFrom(prev, peliasOpenStreetMapImage).
		WithMountedFile("/data/openstreetmap/data.osm.pbf", p.Pelias.Headway.OSMExport.File).
		// OpenStreetMap import also uses WhosOnFirst data
		WithMountedDirectory("/data/whosonfirst", p.Pelias.DownloadWhosOnFirst(ctx)).
		WithExec([]string{"./bin/start"})
}

func (p *PeliasImporter) importPolylines(ctx context.Context, prev *dagger.Container) *dagger.Container {
	return p.importerContainerFrom(prev, peliasPolylinesImage).
		WithMountedFile("/data/polylines/extract.0sv", p.Pelias.Headway.ValhallaPolylines(ctx)).
		// Polylines import also uses WhosOnFirst data
		WithMountedDirectory("/data/whosonfirst", p.Pelias.DownloadWhosOnFirst(ctx)).
		WithExec([]string{"./bin/start"})
}

func (p *Pelias) ElasticsearchData(ctx context.Context) *Artifact {
	importer := p.importer(ctx)

	_, err := importer.ElasticsearchService.Start(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to start elasticsearch service: %w", err))
	}

	// Each step takes the one before it, so the whole import is a single chain
	// rooted at the reset.
	step := importer.reset

	step, err = importer.importSchema(ctx, step).Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to import pelias schema: %w", err))
	}

	step, err = importer.importWhosOnFirst(ctx, step).Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to import WhoseOnFirst data: %w", err))
	}

	if p.OpenAddressesIsEnabled(ctx) {
		step, err = importer.importOpenAddresses(ctx, step).Sync(ctx)
		if err != nil {
			panic(fmt.Errorf("failed to import OpenAddresses data: %w", err))
		}
	}

	step, err = importer.importOpenStreetMap(ctx, step).Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to import OpenStreetMap data: %w", err))
	}

	step, err = importer.importPolylines(ctx, step).Sync(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to import polylines data: %w", err))
	}

	_, err = importer.ElasticsearchService.Stop(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to stop elasticsearch service: %w", err))
	}

	directory := withElasticsearchCacheOwner(slimContainer()).
		// Carrying the last step's marker keeps the export from returning data
		// the current inputs didn't produce.
		WithFile(peliasMarkerPath, step.File(peliasMarkerPath)).
		WithMountedCache("/data-cache", importer.ElasticsearchCacheVolume, peliasCacheOpts()).
		WithExec([]string{"cp", "-r", "/data-cache", "/export"}, dagger.ContainerWithExecOpts{UseEntrypoint: false}).
		Directory("/export")

	return DirectoryArtifact(p.Headway.Area+"-elasticsearch", directory)
}

func (h *Headway) PeliasInitContainer(ctx context.Context) *dagger.Container {
	return downloadContainer().
		WithExec([]string{"mkdir", "-p", "/app"}).
		WithFile("/app/", h.ServiceDir("pelias").File("init_config.sh")).
		WithFile("/app/", h.ServiceDir("pelias").File("init_elastic.sh")).
		WithFile("/app/", h.ServiceDir("pelias").File("init_placeholder.sh")).
		WithDefaultArgs([]string{"echo", "run a specific command"})
}
