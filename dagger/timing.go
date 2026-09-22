package main

import (
	"fmt"
	"os"
	"strings"
	"time"
)

// Timings are scraped out of the build log by bin/build-timings, which
// keys on this prefix.
const timingPrefix = "HEADWAY_TIMING"

// The step the concurrent artifact builds are recorded under. Steps of a phase
// run one after another and so add up to it; what sits below this one does not.
const artifactsTimingStep = "building artifacts"

// record how long a step took, in milliseconds, for the build's timing report.
func recordTiming(label string, start time.Time) {
	fmt.Fprintf(os.Stderr, "%s\t%s\t%d\n", timingPrefix, label, time.Since(start).Milliseconds())
}

// timingLabel names the artifact in the timing report: its stem, without the
// compression suffix that every artifact shares.
func (a *Artifact) timingLabel() string {
	ext := strings.TrimSuffix(strings.TrimSuffix(a.Ext, ".zst"), ".tar")
	return join(".", a.Stem, ext)
}
