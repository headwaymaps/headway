package main

import (
	"context"
	"crypto/sha256"
	"dagger/headway/internal/dagger"
	"fmt"
	"strings"

	"golang.org/x/sync/errgroup"
)

// Zones built at once by BuildTransit when not overridden. Each additional zone
// is another concurrent OTP graph build competing for RAM, so this trades peak
// memory for wall clock.
const defaultMaxConcurrentZones = 3

// ===
// Transit
// ===

type TransitZone struct {
	Headway *Headway
	// Date stamp (YYYY-MM-DD) of the GTFS download that this zone is built from.
	//
	// It's both the cache key for BuildGtfsDir and the artifact versioning
	// scheme, which is what keeps the two honest with each other: the feeds are
	// fetched at most once per day, and the date in the name is by construction
	// the day they were fetched.
	BuildDate string
	// The zone's configuration: `transit/zones/<zone>.json`, as the zone builder
	// writes it. The file name is the zone name, and the document carries the
	// feeds, their credentials and the realtime config.
	TransitFeeds   *dagger.File
	GTFSDir        *dagger.Directory
	OSMExport      *OSMExport
	OTPBuildConfig *dagger.File
}

// Top-level transit orchestrator. Must not be cached: it reads time.Now() to
// decide which day's feeds to build, and a cached call would freeze that at
// whatever day it first ran.
//
// +cache="never"
func (h *Headway) BuildTransit(ctx context.Context,
	transitConfigDir *dagger.Directory,
	// YAML config of credentials for feeds that need one, as a `feeds:` table
	// keyed by Onestop ID. See GtfsCredentialsPath.
	//
	// This has to be passed explicitly: dagger sandboxes module code, so it
	// can't read the host environment and there's no way to pick these up
	// implicitly. bin/build-transit passes
	// `--gtfs-api-keys file://$PWD/gtfs-credentials.yaml`.
	//
	// +optional
	gtfsApiKeys *dagger.Secret,
	// How many zones to build concurrently. Zones are independent, so this is
	// mostly free parallelism - but each one runs its own OTP graph build, and
	// those are memory hungry at planet scale, so peak memory scales with it.
	//
	// Defaults to 3. Pass 1 to get the old sequential behavior back, e.g. when
	// bisecting a failure.
	//
	// +optional
	maxConcurrentZones int) (*dagger.Directory, error) {

	if maxConcurrentZones <= 0 {
		maxConcurrentZones = defaultMaxConcurrentZones
	}

	// Also a cache key for the GTFS download, not just a label. Unlike the
	// dates on the artifacts, this one has to be today's: it's what makes the
	// feeds get re-fetched on a new day.
	gtfsDate := buildDate()

	output := dag.Directory()

	transitFeedsDir := transitConfigDir.Directory("zones")

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
	// First prepare every zone concurrently. This produces all the bboxes that
	// osmium needs for one multi-output extraction from the source PBF.
	//
	// Results are collected by index rather than appended, so the output
	// directory is assembled in the same order no matter who finishes first.
	// That keeps the returned Directory - and so the build cache - stable.
	type zoneResult struct {
		zone       *TransitZone
		stem       string
		zoneBBox   *Bbox
		clipName   string
		gtfs       *Artifact
		graph      *Artifact
		elevations *dagger.Directory
	}
	results := make([]zoneResult, len(transitFeedsFiles))

	group, groupCtx := errgroup.WithContext(ctx)
	group.SetLimit(maxConcurrentZones)

	for i, entry := range transitFeedsFiles {
		group.Go(func() (err error) {
			// Most of the helpers below report failure by panicking. That was
			// survivable when this ran on the main goroutine; from a worker it
			// would take the whole process down mid-flight, with nothing saying
			// which zone was at fault. Convert to an error so errgroup can
			// cancel the siblings and the zone gets named.
			defer func() {
				if r := recover(); r != nil {
					err = fmt.Errorf("transit zone %q failed: %v", entry, r)
				}
			}()

			transitFeedsFile := transitFeedsDir.File(entry)
			zone := h.TransitZone(groupCtx, transitFeedsFile, gtfsDate)
			if otpBuildConfig != nil {
				zone = zone.WithOtpBuildConfig(groupCtx, otpBuildConfig)
			}
			zone = zone.WithGtfsDir(groupCtx, zone.BuildGtfsDir(groupCtx, gtfsDate, gtfsApiKeys))

			name := zone.Name(groupCtx)
			stem := zone.ArtifactStem(groupCtx)
			bbox, err := zone.BBox(groupCtx)
			if err != nil {
				return fmt.Errorf("failed to get bbox for transit zone %q: %w", name, err)
			}
			// The download is keyed on gtfsDate, so dating the artifact from it
			// keeps the name honest about the day the feeds were fetched.
			gtfs := DirectoryArtifact(fmt.Sprintf("%s-gtfs", stem), zone.GTFSDir).Compress()
			gtfs.Date = gtfsDate

			results[i] = zoneResult{
				zone:     zone,
				stem:     stem,
				zoneBBox: bbox,
				clipName: fmt.Sprintf("%s.osm.pbf", name),
				gtfs:     gtfs,
			}
			return nil
		})
	}
	if err := group.Wait(); err != nil {
		return nil, err
	}
	elevationStem := fmt.Sprintf("%s-elevation-tifs", h.Area)

	if len(results) == 0 {
		return DirectoryArtifact(elevationStem, elevations).Compress().AddTo(ctx, output)
	}

	extracts := make([]osmiumExtract, len(results))
	for i, result := range results {
		bbox := result.zoneBBox
		extracts[i] = osmiumExtract{
			Output: result.clipName,
			Bbox:   []float64{bbox.Left, bbox.Bottom, bbox.Right, bbox.Top},
		}
	}
	clippedOSM := h.OSMExport.clipMany(ctx, extracts)

	// Graph builds remain concurrent, but each now consumes its pre-extracted
	// PBF instead of causing another full scan of the source PBF.
	group, groupCtx = errgroup.WithContext(ctx)
	group.SetLimit(maxConcurrentZones)
	for i := range results {
		group.Go(func() (err error) {
			result := &results[i]
			defer func() {
				if r := recover(); r != nil {
					err = fmt.Errorf("transit zone %q failed: %v", result.clipName, r)
				}
			}()

			osmExport := &OSMExport{File: clippedOSM.File(result.clipName)}
			graphStem := fmt.Sprintf("%s-graph", result.stem)
			result.graph = FileArtifact(graphStem, "obj", result.zone.otpGraph(groupCtx, osmExport)).Compress()
			result.elevations = result.zone.Elevations(groupCtx)
			return nil
		})
	}
	if err := group.Wait(); err != nil {
		return nil, err
	}

	artifacts := make([]*Artifact, 0, 2*len(results)+1)
	for _, result := range results {
		artifacts = append(artifacts, result.gtfs, result.graph)
		elevations = elevations.WithDirectory("./", result.elevations)
	}
	artifacts = append(artifacts, DirectoryArtifact(elevationStem, elevations).Compress())

	if err := buildAll(ctx, artifacts); err != nil {
		return nil, err
	}

	for _, artifact := range artifacts {
		output, err = artifact.AddTo(ctx, output)
		if err != nil {
			return nil, err
		}
	}
	return output, nil
}

func zoneFileContents(ctx context.Context, zoneFile *dagger.File) string {
	contents, err := zoneFile.Contents(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to read transit zone file: %w", err))
	}
	return contents
}

func (h *Headway) TransitZone(ctx context.Context, transitFeeds *dagger.File, buildDate string) *TransitZone {
	return &TransitZone{
		Headway:      h,
		BuildDate:    buildDate,
		TransitFeeds: transitFeeds,
	}
}

func (t *TransitZone) ZoneName(ctx context.Context) string {
	fileName, err := t.TransitFeeds.Name(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to get transit feeds name: %w", err))
	}
	return strings.TrimSuffix(fileName, ".json")
}

func (t *TransitZone) WithOtpBuildConfig(ctx context.Context, otpBuildConfig *dagger.File) *TransitZone {
	t.OTPBuildConfig = otpBuildConfig
	return t
}

// Name identifies the zone's build, date included. It names intermediates
// inside the build, where there's no Artifact to carry the date separately.
func (t *TransitZone) Name(ctx context.Context) string {
	return fmt.Sprintf("%s-%s-%s", t.Headway.Area, t.ZoneName(ctx), t.BuildDate)
}

// ArtifactStem identifies the zone without a date: published artifacts get
// their date from the Artifact.
func (t *TransitZone) ArtifactStem(ctx context.Context) string {
	return fmt.Sprintf("%s-%s", t.Headway.Area, t.ZoneName(ctx))
}

func (t *TransitZone) ClippedOsmExport(ctx context.Context) *OSMExport {
	bbox, err := t.BBox(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to get bbox: %w", err))
	}

	return t.Headway.OSMExport.Clip(ctx, bbox)
}

func (t *TransitZone) WithGtfsDir(ctx context.Context, gtfsDir *dagger.Directory) *TransitZone {
	t.GTFSDir = gtfsDir
	return t
}

// Downloads each agency's GTFS zip and repacks them.
//
// buildDate (YYYY-MM-DD) is passed explicitly rather than read off the receiver
// so that it's unambiguously part of dagger's cache key. Every build on the
// same UTC day reuses one download; the first build of a new day re-downloads.
// That's what ties the date stamped into the artifact names to the day the
// feeds were actually fetched.
//
// The TTL is belt-and-suspenders: any same-day reuse is by definition under
// 24h, so it never expires an entry that the date key would still consider
// current.
//
// +cache="24h"
func (t *TransitZone) BuildGtfsDir(ctx context.Context, buildDate string,
	// See BuildTransit.
	//
	// +optional
	gtfsApiKeys *dagger.Secret) *dagger.Directory {
	servicesDir := t.Headway.ServiceDir("gtfs")

	gtfout := t.Headway.Gtfout(ctx)

	container := slimContainer("ca-certificates", "zip", "unzip").
		WithMountedDirectory("/app", servicesDir).
		WithWorkdir("/app").
		WithMountedFile("/usr/local/bin/assume-bikes-allowed", gtfout.File("assume-bikes-allowed")).
		WithMountedFile("/usr/local/bin/download-feeds", gtfout.File("download-feeds"))

	// Mounted as a secret: a picked zone file holds literal feed credentials, so
	// it stays out of the build cache and the logs the way gtfs-credentials.yaml
	// does.
	//
	// A secret's contents are excluded from the exec cache key - that's what
	// keeping them out of the cache means - so the name has to carry the digest.
	// Without it, editing a zone in place leaves every field of this exec
	// identical and the old feeds get served from cache.
	contents := zoneFileContents(ctx, t.TransitFeeds)
	container = container.WithMountedSecret(zoneFilePath, dag.SetSecret(
		fmt.Sprintf("transit-zone-%s-%x", t.ZoneName(ctx), sha256.Sum256([]byte(contents))),
		contents,
	))
	downloadArgs := []string{"download-feeds", "--zone", zoneFilePath, "--output", "downloaded"}

	// A committed zone has the credential fields but not the credentials, so the
	// config still has somewhere to be useful. A zone you filled in yourself
	// carries its own and overrides these.
	if gtfsApiKeys != nil {
		container = container.WithMountedSecret(GtfsCredentialsPath, gtfsApiKeys)
		downloadArgs = append(downloadArgs, "--config", GtfsCredentialsPath)
	}

	return container.
		WithExec(downloadArgs).
		WithExec([]string{"sh", "-c", "./build_gtfs.sh --input downloaded --output ./output"}).
		Directory("./output")
}

// Where the GTFS credential config is mounted for the feed tools to read.
//
// Feeds whose DMFR record has an `authorization` block need a credential to be
// fetched at all - both to measure them for the index and to download them when
// building a zone. The file is YAML: a `feeds:` table keyed by Onestop ID. See
// services/gtfs/gtfout/src/feed_config.rs, and `--write-config-template` for
// generating one.
//
// It arrives as a secret, so the credentials stay out of the build cache and
// logs. It has to be passed in rather than read from the host environment
// because dagger sandboxes module code.
const GtfsCredentialsPath = "/run/secrets/gtfs-credentials.yaml"

// Where a zone file is mounted for download-feeds to read.
//
// Beside the credentials config, and for the same reason: a zone file carries
// the feeds' credentials inline, so it's a secret too.
const zoneFilePath = "/run/secrets/zone.json"

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

// Clones the Transitland Atlas, the catalog we discover GTFS feeds from.
//
// Pinned to a ref so a discovery run is reproducible; override
// HEADWAY_TRANSITLAND_ATLAS_REF to pick up newly cataloged agencies.
func (h *Headway) TransitlandAtlas(ctx context.Context) *dagger.Directory {
	url := getEnvWithDefault("HEADWAY_TRANSITLAND_ATLAS_URL", "https://github.com/transitland/transitland-atlas.git")
	ref := getEnvWithDefault("HEADWAY_TRANSITLAND_ATLAS_REF", "main")
	return dag.Git(url).Branch(ref).Tree()
}

// Where the feed-extents index lives inside the discovery containers.
const gtfsIndexPath = "/cache/feed-extents.gpkg"

// The spatial index of measured GTFS feed extents, for whoever wants the file
// itself; building or refreshing it is a side effect of asking.
//
// Derived data, so it lives in a cache volume rather than git - but it's
// occasionally useful to pull out and inspect with QGIS.
func (h *Headway) GtfsIndex(ctx context.Context,
	// See BuildTransit.
	//
	// +optional
	gtfsApiKeys *dagger.Secret) *dagger.File {
	return h.gtfsIndex(ctx, gtfsApiKeys).
		WithExec([]string{"cp", gtfsIndexPath, "/app/feed-extents.gpkg"}).
		File("/app/feed-extents.gpkg")
}

// A credentials config template listing the feeds that still need one,
// annotated with how each authenticates and where to request a token.
//
// Save it as gtfs-credentials.yaml at the repo root and the build scripts pass
// it through automatically. Passing the credentials you already have carries
// them into the generated file, so regenerating never loses them.
func (h *Headway) GtfsCredentialsTemplate(ctx context.Context,
	// See BuildTransit.
	//
	// +optional
	gtfsApiKeys *dagger.Secret) *dagger.File {
	container := h.gtfsIndex(ctx, gtfsApiKeys)

	args := []string{
		"write-gtfs-index",
		"--atlas-path", "/atlas",
		"--out", gtfsIndexPath,
		// --all names the scope even though --dry-run means nothing is fetched:
		// it's the whole catalog's credentials we're reporting on.
		"--all",
		"--dry-run",
		"--write-config-template", "/app/gtfs-credentials.yaml",
	}
	if gtfsApiKeys != nil {
		// Same file in and out, which is what makes regenerating non-destructive.
		args = append(args, "--config", GtfsCredentialsPath)
	}

	return container.WithExec(args).File("/app/gtfs-credentials.yaml")
}

// Builds or refreshes the index of measured feed extents.
//
// It covers the whole atlas rather than one area. That costs a full download
// pass the first time, but it's what makes the index mean something on its own:
// built per-area into a shared cache, what a query returned depended on which
// areas had been built before it.
//
// The cache volume is what makes this a one-off. Locked sharing because the
// index is SQLite, which takes a single writer.
func (h *Headway) gtfsIndex(ctx context.Context, gtfsApiKeys *dagger.Secret) *dagger.Container {
	// The index is a GeoPackage, which is SQLite - but gtfout compiles it in
	// (see the `bundled` feature in its Cargo.toml), so the container needs
	// nothing for it beyond CA certificates to fetch the feeds over TLS.
	container := slimContainer("ca-certificates").
		WithMountedFile("/usr/local/bin/write-gtfs-index", h.Gtfout(ctx).File("write-gtfs-index")).
		WithMountedDirectory("/atlas", h.TransitlandAtlas(ctx)).
		WithMountedCache("/cache", dag.CacheVolume("headway-gtfs-feed-index"), dagger.ContainerWithMountedCacheOpts{
			Sharing: dagger.CacheSharingModeLocked,
		}).
		WithWorkdir("/app")

	args := []string{
		"write-gtfs-index",
		"--atlas-path", "/atlas",
		"--out", gtfsIndexPath,
		// The index backing every zone's query has to cover the whole catalog;
		// see the comment on this function.
		"--all",
	}
	if gtfsApiKeys != nil {
		container = container.WithMountedSecret(GtfsCredentialsPath, gtfsApiKeys)
		args = append(args, "--config", GtfsCredentialsPath)
	}

	return container.WithExec(args)
}

// Builds Rust GTFS processing tools
// I'm not yet sure how exporting will work in situ. Something akin to:
//
//	dagger -c 'gtfout | file assume-bikes-allowed | export ./assume-bikes-allowed'
func (h *Headway) Gtfout(ctx context.Context) *dagger.Directory {
	container := rustContainer().
		// The cargo workspace root, not just services/gtfs.
		WithMountedDirectory("/repo", h.RepoDir).
		WithWorkdir("/repo").
		WithExec([]string{"cargo", "build", "--release", "--package", "gtfout"})

	return container.Directory("/repo/target/release")
}

// Converts elevation HGT files to TIF format
func (t *TransitZone) Elevations(ctx context.Context) *dagger.Directory {
	bbox, err := t.BBox(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to get bounding box: %w", err))
	}
	return elevations(ctx, bbox, t.Headway)
}

// ===
// OpenTripPlanner
// ===

func otpBaseContainer(ctx context.Context) *dagger.Container {
	return dag.Container().
		From("opentripplanner/opentripplanner:2.9.0")
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
	return t.otpGraph(ctx, osmExport)
}

func (t *TransitZone) otpGraph(ctx context.Context, osmExport *OSMExport) *dagger.File {

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
