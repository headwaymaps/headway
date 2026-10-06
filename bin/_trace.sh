#!/bin/bash
# Trace a build script with OpenTelemetry, and summarize the trace when it ends.
#
#     source bin/_trace.sh
#     trace_start <name>         # the root span, which the trace is filed under
#     traced <step> <command...> # a step of the summary, nesting whatever
#                                # spans the command emits - dagger's included
#     trace_finish               # usually from an EXIT trap
#
# For the length of the script, an OpenTelemetry Collector running in docker
# stores the trace in data/traces/<name>/ as OTLP JSON.

if ! command -v otel-cli > /dev/null; then
    echo "Error: otel-cli is required to trace the build - see BUILD.md" >&2
    exit 1
fi

TRACE_COLLECTOR_IMAGE="otel/opentelemetry-collector:0.162.0"

TRACE_ENDPOINT=""
TRACE_FILE=""
TRACE_COLLECTOR=""
TRACE_ROOT_DIR=""
TRACE_ROOT_PID=""

function trace_start() {
    local name=$1 port
    port=$((20000 + RANDOM % 40000))
    TRACE_ENDPOINT="127.0.0.1:${port}"
    local trace_dir="data/traces/${name}" trace_name
    trace_name="$(date -u +%Y-%m-%dT%H%M%SZ).jsonl"
    TRACE_FILE="${trace_dir}/${trace_name}"
    mkdir -p "$trace_dir"

    TRACE_COLLECTOR="headway-trace-${port}"
    # As our own user, so the trace isn't written as the image's.
    docker run --detach --name "$TRACE_COLLECTOR" --user "$(id -u):$(id -g)" \
        --publish "${TRACE_ENDPOINT}:4317" \
        --volume "${PWD}/${trace_dir}:/traces" \
        --volume "${PWD}/bin/_trace-collector.yaml:/etc/otelcol/config.yaml:ro" \
        --env HEADWAY_TRACE_FILE="$trace_name" \
        "$TRACE_COLLECTOR_IMAGE" --config /etc/otelcol/config.yaml > /dev/null
    # docker accepts connections on the published port before the collector
    # inside is listening, so wait on the collector itself.
    until docker logs "$TRACE_COLLECTOR" 2>&1 | grep -q "Everything is ready"; do
        if [ "$(docker inspect --format '{{.State.Running}}' "$TRACE_COLLECTOR")" != true ]; then
            echo "Error: the trace collector failed to start:" >&2
            docker logs "$TRACE_COLLECTOR" >&2
            docker rm "$TRACE_COLLECTOR" > /dev/null
            exit 1
        fi
        sleep 0.2
    done

    # Read by dagger, which nests its spans under $TRACEPARENT.
    export OTEL_EXPORTER_OTLP_ENDPOINT="http://${TRACE_ENDPOINT}"
    export OTEL_EXPORTER_OTLP_PROTOCOL=grpc

    # The root span's socket lives in /tmp because a unix socket path is limited
    # to about 100 bytes, which macOS's $TMPDIR nearly uses up on its own.
    TRACE_ROOT_DIR=$(mktemp -d /tmp/headway-span.XXXXXX)
    local traceparent_file="${TRACE_ROOT_DIR}/traceparent"
    otel-cli span background --endpoint "$TRACE_ENDPOINT" --insecure \
        --service headway --name "$name" --attrs headway.step=true \
        --sockdir "$TRACE_ROOT_DIR" --timeout 48h --tp-print > "$traceparent_file" &
    TRACE_ROOT_PID=$!
    until grep -q '^TRACEPARENT=' "$traceparent_file" 2> /dev/null; do
        sleep 0.1
    done
    TRACEPARENT=$(sed -n 's/^TRACEPARENT=//p' "$traceparent_file")
    export TRACEPARENT
}

function traced() {
    local name=$1
    shift
    # --fail, because otherwise a span that fails to send hides how $@ exited.
    otel-cli exec --fail --endpoint "$TRACE_ENDPOINT" --insecure \
        --service headway --name "$name" --attrs headway.step=true -- "$@"
}

function trace_finish() {
    [ -n "$TRACE_ROOT_PID" ] || return 0
    otel-cli span end --sockdir "$TRACE_ROOT_DIR"
    wait "$TRACE_ROOT_PID" || true
    # The collector writes out what it holds as it shuts down.
    docker stop "$TRACE_COLLECTOR" > /dev/null
    docker rm "$TRACE_COLLECTOR" > /dev/null
    rm -rf "$TRACE_ROOT_DIR"
    TRACE_ROOT_PID=""

    gzip "$TRACE_FILE"
    bin/build-timings "${TRACE_FILE}.gz"
}
