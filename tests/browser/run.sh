#!/bin/bash
# Run the Playwright browser tests against running services.
#
# Assumes the stack is already up, like tests/integration/run-tests.sh:
#
#   bin/start-services --no-follow-logs builds/Bogota
#   bin/wait-for-services
#
# Arguments are passed through to `playwright test`, e.g.
#
#   tests/browser/run.sh --headed
#   tests/browser/run.sh -g "routing"

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

FRONTEND_URL="${FRONTEND_URL:-http://localhost:8080}"
export FRONTEND_URL

if ! curl -fsS "$FRONTEND_URL" >/dev/null 2>&1; then
    echo "❌ Error: nothing serving $FRONTEND_URL"
    echo "   Start the stack with: bin/start-services --no-follow-logs builds/Bogota"
    exit 1
fi

cd "$SCRIPT_DIR"

# Playwright and its browser are dev dependencies of this directory only, so a
# fresh checkout needs them before the first run.
if [ ! -x node_modules/.bin/playwright ]; then
    echo "📦 Installing browser test dependencies..."
    yarn install
    npx playwright install chrome
fi

echo "🧪 Running browser tests against $FRONTEND_URL..."
npx playwright test "$@"
