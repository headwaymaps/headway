# Integration Tests

End-to-end integration tests for the Headway stack.

## Quick Start

### One-shot testing (CI mode)

Run everything at once - starts services, runs tests, cleans up:

BEWARE: This will destroy your headway docker volumes.
```bash
bin/test/integration-tests-oneshot
```

This is what CI uses. Services are automatically cleaned up when tests complete.

### Interactive testing (development mode)

When iterating on tests, keep services running between test runs:

```bash
# 1. Start services once
bin/start-services --no-follow-logs builds/Bogota

# 2. Wait for services to be ready
bin/wait-for-services

# 3. Run tests against the build's smoke-test.env (can repeat this step)
bin/test/integration-tests builds/Bogota

# Edit test scripts and re-run as needed...
bin/test/integration-tests builds/Bogota

# 4. Stop services when done
# BEWARE: This will destroy your headway docker volumes.
bin/stop-and-remove-services builds/Bogota
```
