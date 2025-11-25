#!/bin/bash
# Common test library functions

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0

# Helper function to run a JSON test with jq validations
run_jq_test() {
    local test_name=$1
    local url=$2
    shift 2
    local validations=("$@")

    # Check if jq is available
    if ! command -v jq &> /dev/null; then
        echo "Error: jq is required but not installed"
        echo "Install with: apt-get install jq (Debian/Ubuntu) or brew install jq (macOS)"
        exit 1
    fi

    echo -n "  Testing $test_name... "

    # Make request
    response=$(curl -s -w "\n%{http_code}" "$url")
    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | sed '$ d')

    # Check status code
    if [ "$http_code" != "200" ]; then
        echo "FAIL (HTTP $http_code)"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi

    # Check if valid JSON (if validations exist)
    if [ ${#validations[@]} -gt 0 ]; then
        if ! echo "$body" | jq empty 2>/dev/null; then
            echo "FAIL (invalid JSON)"
            echo "Response body:"
            echo "$body" | head -100
            TESTS_FAILED=$((TESTS_FAILED + 1))
            return 1
        fi

        # Run custom validations
        for validation in "${validations[@]}"; do
            if ! echo "$body" | jq -e "$validation" > /dev/null 2>&1; then
                echo "FAIL (validation failed: $validation)"
                echo "Response body:"
                echo "$body" | jq '.' 2>/dev/null || echo "$body"
                TESTS_FAILED=$((TESTS_FAILED + 1))
                return 1
            fi
        done
    fi

    echo "PASS"
    TESTS_PASSED=$((TESTS_PASSED + 1))
    return 0
}

# Helper function to check binary content
run_binary_test() {
    local test_name=$1
    local url=$2
    local expected_content_type=$3
    local min_size=${4:-100}

    echo -n "  Testing $test_name... "

    # Make request and save response
    response=$(curl -s -w "\n%{http_code}\n%{content_type}\n%{size_download}" "$url")

    # Parse response
    http_code=$(echo "$response" | tail -n 3 | head -n 1)
    content_type=$(echo "$response" | tail -n 2 | head -n 1)
    size=$(echo "$response" | tail -n 1)

    # Check status code
    if [ "$http_code" != "200" ]; then
        echo "FAIL (HTTP $http_code)"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi

    # Check content type if specified
    if [ -n "$expected_content_type" ] && ! echo "$content_type" | grep -q "$expected_content_type"; then
        echo "FAIL (wrong content-type: $content_type)"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi

    # Check size
    if [ "$size" -lt "$min_size" ]; then
        echo "FAIL (size too small: $size bytes)"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi

    echo "PASS (${size} bytes)"
    TESTS_PASSED=$((TESTS_PASSED + 1))
    return 0
}

# Print test summary
print_test_summary() {
    local suite_name=$1
    echo ""
    echo "$suite_name tests complete: $TESTS_PASSED passed, $TESTS_FAILED failed"

    if [ $TESTS_FAILED -gt 0 ]; then
        exit 1
    fi

    exit 0
}
