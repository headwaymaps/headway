package main

import (
	"context"
	"dagger/headway/internal/dagger"
	"fmt"
	"strings"
	"time"

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
	//
	// Published builds accumulate side by side as e.g.
	// planet-puget_sound-2026-07-28.graph.obj.zst, and bin/link-latest-transit
	// parses the date back out to symlink the newest one to
	// PugetSound.graph.obj.zst.
	BuildDate      string
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
	// Env-file (KEY=VALUE lines) of API keys for feeds whose CSV row sets
	// urls.authentication_type, named HEADWAY_GTFS_API_KEY_<mdb_source_id>.
	//
	// This has to be passed explicitly: dagger sandboxes module code, so it
	// can't read the host environment and there's no way to pick these up
	// implicitly. bin/build-transit passes `--gtfs-api-keys file://$PWD/.bin-env`.
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

	// UTC so the day boundary doesn't depend on where the build runs - this
	// date is a cache key, not just a label.
	buildDate := time.Now().UTC().Format("2006-01-02")

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
	// First prepare every zone concurrently. This produces all the bboxes that
	// osmium needs for one multi-output extraction from the source PBF.
	//
	// Results are collected by index rather than appended, so the output
	// directory is assembled in the same order no matter who finishes first.
	// That keeps the returned Directory - and so the build cache - stable.
	type zoneResult struct {
		zone       *TransitZone
		zoneBBox   *Bbox
		clipName   string
		gtfsName   string
		gtfsFile   *dagger.File
		graphName  string
		graphFile  *dagger.File
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
			zone := h.TransitZone(groupCtx, transitFeedsFile, buildDate)
			if otpBuildConfig != nil {
				zone = zone.WithOtpBuildConfig(groupCtx, otpBuildConfig)
			}
			zone = zone.WithGtfsDir(groupCtx, zone.BuildGtfsDir(groupCtx, buildDate, gtfsApiKeys))

			name := zone.Name(groupCtx)
			bbox, err := zone.BBox(groupCtx)
			if err != nil {
				return fmt.Errorf("failed to get bbox for transit zone %q: %w", name, err)
			}
			results[i] = zoneResult{
				zone:      zone,
				zoneBBox:  bbox,
				clipName:  fmt.Sprintf("%s.osm.pbf", name),
				gtfsName:  fmt.Sprintf("%s.gtfs.tar.zst", name),
				gtfsFile:  compressDir(zone.GTFSDir),
				graphName: fmt.Sprintf("%s.graph.obj.zst", name),
			}
			return nil
		})
	}
	if err := group.Wait(); err != nil {
		return nil, err
	}
	if len(results) == 0 {
		return output.WithFile(
			fmt.Sprintf("%s-%s.elevation-tifs.tar.zst", h.Area, buildDate),
			compressDir(elevations),
		), nil
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
			result.graphFile = compressFile(result.zone.otpGraph(groupCtx, osmExport))
			result.elevations = result.zone.Elevations(groupCtx)
			return nil
		})
	}
	if err := group.Wait(); err != nil {
		return nil, err
	}

	for _, result := range results {
		output = output.WithFile(result.gtfsName, result.gtfsFile)
		output = output.WithFile(result.graphName, result.graphFile)
		elevations = elevations.WithDirectory("./", result.elevations)
	}
	output = output.WithFile(fmt.Sprintf("%s-%s.elevation-tifs.tar.zst", h.Area, buildDate), compressDir(elevations))

	return output, nil
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
	return strings.TrimSuffix(fileName, ".gtfs_feeds.csv")
}

func (t *TransitZone) WithOtpBuildConfig(ctx context.Context, otpBuildConfig *dagger.File) *TransitZone {
	t.OTPBuildConfig = otpBuildConfig
	return t
}

func (t *TransitZone) Name(ctx context.Context) string {
	return fmt.Sprintf("%s-%s-%s", t.Headway.Area, t.ZoneName(ctx), t.BuildDate)
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

	assumeBikesAllowed := t.Headway.Gtfout(ctx).File("assume-bikes-allowed")

	container := dag.Container().
		From("python:3")
	container = WithAptPackages(container, "zip").
		WithExec([]string{"pip", "install", "requests"}).
		WithMountedDirectory("/app", servicesDir).
		WithWorkdir("/app").
		WithMountedFile("/usr/local/bin/assume-bikes-allowed", assumeBikesAllowed).
		WithMountedFile("gtfs_feeds.csv", t.TransitFeeds)
	if gtfsApiKeys != nil {
		container = container.WithMountedSecret(GtfsApiKeysPath, gtfsApiKeys)
	}
	return container.
		WithExec([]string{"sh", "-c", "./download_gtfs_feeds.py --output=downloaded < gtfs_feeds.csv"}).
		WithExec([]string{"sh", "-c", "./build_gtfs.sh --input downloaded --output ./output"}).
		Directory("./output")
}

// Where BuildGtfsDir mounts the GTFS API key env-file for download_gtfs_feeds.py
// to read. Mounted as a secret so the keys stay out of the build cache and logs.
const GtfsApiKeysPath = "/run/secrets/gtfs-api-keys"

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
	downloadUrl := getEnvWithDefault("HEADWAY_MOBILITYDB_URL", "https://storage.googleapis.com/storage/v1/b/mdb-csv/o/sources.csv?alt=media")
	return downloadFile(downloadUrl)
}

// Enumerates GTFS feeds for a given area by filtering the mobility database
func (h *Headway) NearbyGtfsFeeds(ctx context.Context) *dagger.File {
	if h.Area == "" {
		panic("Area is required for GTFS enumeration")
	}

	bbox, err := h.BBox(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to get bounding box for area %s: %w", h.Area, err))
	}

	mobilityDb := h.GtfsGetMobilitydb(ctx)
	servicesDir := h.ServiceDir("gtfs")

	container := dag.Container().
		From("python:3").
		WithMountedDirectory("/app", servicesDir).
		WithMountedFile("/app/sources.csv", mobilityDb).
		WithWorkdir("/app").
		WithExec([]string{"sh", "-c", fmt.Sprintf("./filter_feeds.py --bbox='%s' < sources.csv > nearby_gtfs_feeds.csv", bbox.SpaceSeparated())})

	return container.File("/app/nearby_gtfs_feeds.csv")
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
