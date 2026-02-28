#!/bin/sh
# Run model parity tests filtered by category.
#
# Usage:
#   sh scripts/run_model_parity_by_category.sh [category]
#
# Categories:
#   all       - Run all models (default)
#   examples  - 45 example models
#   tickets   - 110 ticket models
#   tests     - 193 test models
#   realworld - 1,100 real-world models
#
# Output is written to tests/model_parity_{category}/.
# All MODEL_PARITY_* and JAVA_PARITY_* env vars are forwarded.

set -eu

CATEGORY=${1:-all}
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)

case "$CATEGORY" in
  all)       INCLUDE="" ; OUTPUT="tests/model_parity" ;;
  examples)  INCLUDE="examples" ; OUTPUT="tests/model_parity_examples" ;;
  tickets)   INCLUDE="tickets" ; OUTPUT="tests/model_parity_tickets" ;;
  tests)     INCLUDE="tests" ; OUTPUT="tests/model_parity_tests" ;;
  realworld) INCLUDE="realworld" ; OUTPUT="tests/model_parity_realworld" ;;
  *)
    echo "Unknown category: $CATEGORY" >&2
    echo "Valid categories: all, examples, tickets, tests, realworld" >&2
    exit 1
    ;;
esac

echo "Running model parity for category: $CATEGORY"
echo "  include filter: ${INCLUDE:-<none>}"
echo "  output dir:     $OUTPUT"

DEFAULT_EXCLUDE_FILE="$SCRIPT_DIR/../tests/model_parity_full/java_exclude.txt"
if [ -z "${JAVA_PARITY_EXCLUDE_FILE:-}" ] && [ -f "$DEFAULT_EXCLUDE_FILE" ]; then
  JAVA_PARITY_EXCLUDE_FILE="$DEFAULT_EXCLUDE_FILE"
  echo "  exclude file:   $JAVA_PARITY_EXCLUDE_FILE"
fi

JAVA_PARITY_INCLUDE="$INCLUDE" \
  sh "$SCRIPT_DIR/run_model_parity_elk_vs_rust.sh" external/elk-models "$OUTPUT"
