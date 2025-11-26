#!/bin/bash
# Stop docker compose services

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROJECT_ROOT"

COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.yaml}"

echo "Stopping services..."
docker compose -f "$COMPOSE_FILE" down --volumes
echo "Services stopped and volumes removed."
