#!/bin/sh
# Full corpus verification using matching JSON inputs and a corrected Java reference.
# JAVA_TRACE_CLASSPATH must include the pinned compiled ELK modules, test module,
# and runtime dependencies. PHASE_INPUT_MANIFEST must cover the entire corpus.
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
: "${JAVA_TRACE_CLASSPATH:?Set JAVA_TRACE_CLASSPATH to the pinned Java ELK classpath}"
: "${PHASE_INPUT_MANIFEST:?Set PHASE_INPUT_MANIFEST to the complete input TSV}"
exec python3 "$SCRIPT_DIR/run_full_trace_parity.py" \
    --models-root "${1:-external/elk-models}" \
    --output "${2:-tests/model_parity/full-phase}" \
    --inputs "$PHASE_INPUT_MANIFEST" \
    --java-classpath "$JAVA_TRACE_CLASSPATH"
