package main

import (
	"context"
	"strings"

	"go.opentelemetry.io/otel/attribute"
	"go.opentelemetry.io/otel/trace"
)

// Marks the spans that bin/build-timings reports, out of the many dagger emits.
const stepAttribute = "headway.step"

// startStep opens a span for the build's timing summary.
func startStep(ctx context.Context, name string) (context.Context, trace.Span) {
	return Tracer().Start(ctx, name, trace.WithAttributes(attribute.Bool(stepAttribute, true)))
}

// stepName names the artifact in the timing summary: its stem, without the
// compression suffix that every artifact shares.
func (a *Artifact) stepName() string {
	ext := strings.TrimSuffix(strings.TrimSuffix(a.Ext, ".zst"), ".tar")
	return join(".", a.Stem, ext)
}
