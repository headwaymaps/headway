package main

import (
	"context"
	"dagger/headway/internal/dagger"
	"fmt"
	"path/filepath"
	"sort"

	"golang.org/x/sync/errgroup"
)

const defaultMaxConcurrentZones = 3

type TransitZone struct {
	Headway *Headway

	BuildDate string

	Zone string

	TransitFeeds   *dagger.File
	GTFSDir        *dagger.Directory
	OSMExport      *OSMExport
	OTPBuildConfig *dagger.File
}

// Select today's feeds on every invocation.
// +cache="never"
func (h *Headway) BuildTransit(ctx context.Context,
	// +ignore=["**/.env", "**/gtfs-secrets.json"]
	transitConfigDir *dagger.Directory,
	// +optional
	gtfsSecrets *dagger.Secret,
	// +optional
	maxConcurrentZones int) (*dagger.Directory, error) {

	if maxConcurrentZones <= 0 {
		maxConcurrentZones = defaultMaxConcurrentZones
	}

	gtfsDate := buildDate()

	output := dag.Directory()

	otpBuildConfig := (*dagger.File)(nil)
	otpConfigExists, err := transitConfigDir.Exists(ctx, "otp-build-config.json")
	if err != nil {
		panic(fmt.Errorf("failed to check if otp-build-config.json exists: %w", err))
	}
	if otpConfigExists {
		otpBuildConfig = transitConfigDir.File("otp-build-config.json")
	}
	elevations := dag.Directory()
	zoneFiles, err := transitZoneFiles(ctx, transitConfigDir)
	if err != nil {
		return nil, err
	}

	type zoneResult struct {
		zone       *TransitZone
		stem       string
		zoneBBox   *Bbox
		clipName   string
		gtfs       *Artifact
		graph      *Artifact
		elevations *dagger.Directory
	}
	results := make([]zoneResult, len(zoneFiles))

	group, groupCtx := errgroup.WithContext(ctx)
	group.SetLimit(maxConcurrentZones)

	for i, entry := range zoneFiles {
		group.Go(func() (err error) {

			defer func() {
				if r := recover(); r != nil {
					err = fmt.Errorf("transit zone %q failed: %v", entry.name, r)
				}
			}()

			transitFeedsFile := transitConfigDir.File(entry.path)
			zone := h.TransitZone(groupCtx, entry.name, transitFeedsFile, gtfsDate)
			if otpBuildConfig != nil {
				zone = zone.WithOtpBuildConfig(groupCtx, otpBuildConfig)
			}
			zone = zone.WithGtfsDir(groupCtx, zone.BuildGtfsDir(groupCtx, gtfsDate, gtfsSecrets))

			name := zone.Name(groupCtx)
			stem := zone.ArtifactStem(groupCtx)
			bbox, err := zone.BBox(groupCtx)
			if err != nil {
				return fmt.Errorf("failed to get bbox for transit zone %q: %w", name, err)
			}

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

type transitZoneFile struct {
	name string

	path string
}

func transitZoneFiles(ctx context.Context, transitConfigDir *dagger.Directory) ([]transitZoneFile, error) {
	paths, err := transitConfigDir.Glob(ctx, "*/zone.json")
	if err != nil {
		return nil, fmt.Errorf("failed to list transit zones: %w", err)
	}

	sort.Strings(paths)
	zones := make([]transitZoneFile, 0, len(paths))
	for _, path := range paths {
		zones = append(zones, transitZoneFile{name: filepath.Dir(path), path: path})
	}
	return zones, nil
}

func (h *Headway) TransitZone(ctx context.Context, zone string, transitFeeds *dagger.File, buildDate string) *TransitZone {
	return &TransitZone{
		Headway:      h,
		Zone:         zone,
		BuildDate:    buildDate,
		TransitFeeds: transitFeeds,
	}
}

func (t *TransitZone) ZoneName(ctx context.Context) string {
	return t.Zone
}

func (t *TransitZone) WithOtpBuildConfig(ctx context.Context, otpBuildConfig *dagger.File) *TransitZone {
	t.OTPBuildConfig = otpBuildConfig
	return t
}

func (t *TransitZone) Name(ctx context.Context) string {
	return fmt.Sprintf("%s-%s-%s", t.Headway.Area, t.ZoneName(ctx), t.BuildDate)
}

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

// buildDate invalidates the download cache each UTC day.
// +cache="24h"
func (t *TransitZone) BuildGtfsDir(ctx context.Context, buildDate string,
	// +optional
	gtfsSecrets *dagger.Secret) *dagger.Directory {
	servicesDir := t.Headway.ServiceDir("gtfs")

	gtfout := t.Headway.Gtfout(ctx)

	container := slimContainer("ca-certificates", "zip", "unzip").
		WithMountedDirectory("/app", servicesDir).
		WithWorkdir("/app").
		WithMountedFile("/usr/local/bin/assume-bikes-allowed", gtfout.File("assume-bikes-allowed")).
		WithMountedFile("/usr/local/bin/download-feeds", gtfout.File("download-feeds"))

	container = container.WithMountedFile(zoneFilePath, t.TransitFeeds)

	downloadArgs := []string{"download-feeds", "--zone", zoneFilePath, "--output", "downloaded"}

	if gtfsSecrets != nil {
		container = container.WithMountedSecret(GtfsSecretsPath, gtfsSecrets)
		downloadArgs = append(downloadArgs, "--credentials-file", GtfsSecretsPath)
	}

	return container.
		WithExec(downloadArgs).
		WithExec([]string{"sh", "-c", "./build_gtfs.sh --input downloaded --output ./output"}).
		Directory("./output")
}

// gtfs-secrets.json, in transitland's secrets.json format, which is how gtfout
// reads credentials.
const GtfsSecretsPath = "/run/secrets/gtfs-secrets.json"

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

// Downloads the GTFS feed-extents index from the headway-data repository.
//
// +cache="never"
func (h *Headway) DownloadGtfsIndex(ctx context.Context) (*dagger.File, error) {
	commit, err := dag.Git(headwayDataRepo).Branch("main").Commit(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to resolve %s main: %w", headwayDataRepo, err)
	}
	return h.DownloadGtfsIndexAtCommit(ctx, commit), nil
}

// Downloads the GTFS feed-extents index at a specific headway-data commit
func (h *Headway) DownloadGtfsIndexAtCommit(ctx context.Context, commit string) *dagger.File {
	url := getEnvWithDefault("HEADWAY_GTFS_INDEX_URL",
		fmt.Sprintf("%s/raw/%s/gtfs/feed-extents.gpkg", headwayDataRepo, commit))
	return downloadFile(url)
}

func (h *Headway) Gtfout(ctx context.Context) *dagger.Directory {
	container := rustContainer().
		WithMountedDirectory("/repo", h.RepoDir).
		WithWorkdir("/repo").
		WithExec([]string{"cargo", "build", "--release",
			"--package", "gtfout"})

	return container.Directory("/repo/target/release")
}

func (t *TransitZone) Elevations(ctx context.Context) *dagger.Directory {
	bbox, err := t.BBox(ctx)
	if err != nil {
		panic(fmt.Errorf("failed to get bounding box: %w", err))
	}
	return elevations(ctx, bbox, t.Headway)
}

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

	return container
}

func (h *Headway) OtpInitContainer(ctx context.Context) *dagger.Container {
	return downloadContainer().
		WithFile("/usr/local/bin/zone-router-config", h.Gtfout(ctx).File("zone-router-config")).
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
