package main

import (
	"context"
	"dagger/headway/internal/dagger"
	"fmt"
	"strings"
	"time"
)

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

func (h *Headway) BuildTransit(ctx context.Context,
	transitConfigDir *dagger.Directory) (*dagger.Directory, error) {

	cacheKey := time.Now().Format("2006-01-02")

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
