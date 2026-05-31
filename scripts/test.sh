#!/bin/bash
set -e

echo "🧪 AI Gateway Test Suite"
echo "========================"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

case "${1:-all}" in
  unit)
    echo -e "${YELLOW}Running unit tests...${NC}"
    cargo test --workspace --lib -- --nocapture
    echo -e "${GREEN}✅ Unit tests passed!${NC}"
    ;;

  integration)
    echo -e "${YELLOW}Running integration tests...${NC}"
    echo "Requires PostgreSQL and Redis running locally or TEST_DATABASE_URL set."
    cargo test --package gateway --test integration_test -- --nocapture
    echo -e "${GREEN}✅ Integration tests passed!${NC}"
    ;;

  docker)
    echo -e "${YELLOW}Running tests in Docker...${NC}"
    docker-compose -f docker-compose.test.yml up --build --abort-on-container-exit --exit-code-from test-runner
    RESULT=$?
    docker-compose -f docker-compose.test.yml down -v
    if [ $RESULT -eq 0 ]; then
      echo -e "${GREEN}✅ Docker tests passed!${NC}"
    else
      echo -e "${RED}❌ Docker tests failed!${NC}"
      exit 1
    fi
    ;;

  all)
    echo -e "${YELLOW}Running all tests...${NC}"
    echo ""
    $0 unit
    echo ""
    $0 integration
    echo ""
    echo -e "${GREEN}✅ All tests passed!${NC}"
    ;;

  *)
    echo "Usage: $0 {unit|integration|docker|all}"
    echo ""
    echo "  unit         - Run unit tests (no external deps)"
    echo "  integration  - Run integration tests (needs DB + Redis)"
    echo "  docker       - Run all tests in Docker containers"
    echo "  all          - Run unit + integration tests"
    exit 1
    ;;
esac

