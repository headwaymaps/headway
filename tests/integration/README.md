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
# 1. Start services once
./tests/integration/start-services.sh

# 2. Wait for services to be ready
./tests/integration/wait-for-services.sh

# 3. Run tests (can repeat this step)
./tests/integration/run-tests.sh

# Edit test scripts and re-run as needed...
./tests/integration/run-tests.sh

# 4. Stop services when done
# BEWARE: This will destroy your headway docker volumes.
./tests/integration/stop-services.sh
```
