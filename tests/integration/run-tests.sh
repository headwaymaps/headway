#!/bin/bash
# Run integration tests against running services
# Assumes services are already started (use start-services.sh first)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROJECT_ROOT"

FRONTEND_URL="${FRONTEND_URL:-http://localhost:8080}"

echo "========================================"
echo "Running Integration Tests"
echo "========================================"
echo "Frontend URL: $FRONTEND_URL"
echo ""

TESTS_PASSED=0
TESTS_FAILED=0

# Export FRONTEND_URL for test scripts
export FRONTEND_URL

# Run tileserver tests
echo "Running tileserver tests..."
if "$SCRIPT_DIR/test-tileserver.sh"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo "Tileserver tests FAILED"
fi

# Run geosearch tests
echo ""
echo "Running geosearch tests..."
if "$SCRIPT_DIR/test-geosearch.sh"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo "Geosearch tests FAILED"
fi

# Run routing tests
echo ""
echo "Running routing tests..."
if "$SCRIPT_DIR/test-routing.sh"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo "Routing tests FAILED"
fi

echo ""
echo "========================================"
echo "Test Summary"
echo "========================================"
echo "Test suites passed: $TESTS_PASSED"
echo "Test suites failed: $TESTS_FAILED"
echo ""

if [ $TESTS_FAILED -gt 0 ]; then
    echo "FAILURE: Some tests failed"
    echo ""
    echo "To view logs, run:"
    echo "  docker-compose logs"
    exit 1
else
    echo "SUCCESS: All tests passed!"
    exit 0
fi
