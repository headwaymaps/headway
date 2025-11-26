#!/bin/bash
# Start docker compose services for integration testing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROJECT_ROOT"

COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.yaml}"

echo "========================================"
echo "Starting Headway Services"
echo "========================================"
echo "Project root: $PROJECT_ROOT"
echo "Compose file: $COMPOSE_FILE"
echo ""

source .env.sh
echo "Area: ${HEADWAY_AREA}"
echo "Data directory: data/${HEADWAY_AREA}"
echo ""

# Check if data directory exists
if [ ! -d "data/${HEADWAY_AREA}" ]; then
    echo "Error: Data directory data/${HEADWAY_AREA} not found"
    echo "Please run: bin/build builds/${HEADWAY_AREA,,}"
    exit 1
fi

# Start services
echo "Starting docker compose services..."
docker compose -f "$COMPOSE_FILE" up -d

echo ""
echo "Services started! Use wait-for-services.sh to check readiness."
echo ""
echo "To stop services, run:"
echo "  docker compose down --volumes"
