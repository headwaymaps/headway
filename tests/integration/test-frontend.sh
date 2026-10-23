#!/bin/bash
# Test that every asset the served index.html loads exists. A stale cached index.html references
# chunks from a previous build, which breaks the app only once it lazy-loads one.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/_test-lib.sh"

ASSET_PATTERN='assets/[A-Za-z0-9_.-]+\.(js|css)'

echo "Testing Frontend..."
echo ""

echo -n "  Testing assets loaded by index.html... "

# No cache busting: test what a browser gets, CDN cache included.
index_html=$(curl -s "$HEADWAY_PUBLIC_URL/")
entry_scripts=$(grep -oE 'assets/[A-Za-z0-9_.-]+\.js' <<<"$index_html")

# The entry script lists every lazily loaded chunk.
assets=($(
    {
        grep -oE "$ASSET_PATTERN" <<<"$index_html"
        for script in $entry_scripts; do
            curl -s "$HEADWAY_PUBLIC_URL/$script" | grep -oE "$ASSET_PATTERN"
        done
    } | sort -u
))

missing=()
for asset in "${assets[@]}"; do
    http_code=$(curl -s -o /dev/null -w "%{http_code}" "$HEADWAY_PUBLIC_URL/$asset")
    [ "$http_code" = 200 ] || missing+=("$asset ($http_code)")
done

if [ ${#assets[@]} -eq 0 ]; then
    echo "FAIL (no assets found in index.html)"
    TESTS_FAILED=$((TESTS_FAILED + 1))
elif [ ${#missing[@]} -gt 0 ]; then
    echo "FAIL (${#missing[@]} of ${#assets[@]} missing - stale index.html?)"
    printf '    %s\n' "${missing[@]}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
else
    echo "PASS (${#assets[@]} assets)"
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi

print_test_summary "Frontend"
