# Integration Tests

End-to-end integration tests for the Headway stack.

## Quick Start

### One-shot testing (CI mode)

Run everything at once - starts services, runs tests, cleans up:

BEWARE: This will destroy your headway docker volumes.
```bash
./tests/integration/run-integration-tests.sh
```

This is what CI uses. Services are automatically cleaned up when tests complete.

### Interactive testing (development mode)

When iterating on tests, keep services running between test runs:

```bash
# 1. Link the latest transit artifacts (only needed after a fresh bin/build-transit)
TRANSIT_DATA_ROOT=./data bin/link-latest-transit builds/Bogota

# 2. Start services once
bin/start-services --no-follow-logs builds/Bogota

# 3. Wait for services to be ready
bin/wait-for-services

# 4. Run tests (can repeat this step)
./tests/integration/run-tests.sh

# Edit test scripts and re-run as needed...
./tests/integration/run-tests.sh

# 5. Stop services when done
# BEWARE: This will destroy your headway docker volumes.
bin/stop-and-remove-services builds/Bogota
```
