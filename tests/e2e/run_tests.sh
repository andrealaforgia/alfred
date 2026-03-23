#!/usr/bin/env bash
#
# Run Alfred end-to-end tests inside Docker.
#
# Usage:
#   ./tests/e2e/run_tests.sh           # Build and run all e2e tests
#   ./tests/e2e/run_tests.sh --rebuild  # Force rebuild the Docker image
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROJECT_ROOT"

echo "=== Alfred End-to-End Tests ==="
echo ""

# Build and run via docker compose
if [[ "${1:-}" == "--rebuild" ]]; then
    echo "Forcing rebuild..."
    docker compose -f tests/e2e/docker-compose.yml build --no-cache
fi

echo "Building Docker image and running tests..."
echo ""

docker compose -f tests/e2e/docker-compose.yml up --build --abort-on-container-exit --exit-code-from e2e

EXIT_CODE=$?

# Cleanup
docker compose -f tests/e2e/docker-compose.yml down --remove-orphans 2>/dev/null || true

if [ $EXIT_CODE -eq 0 ]; then
    echo ""
    echo "=== All e2e tests passed ==="
else
    echo ""
    echo "=== e2e tests FAILED (exit code: $EXIT_CODE) ==="
fi

exit $EXIT_CODE
