#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)

JAVA_LAYERED_GRAPH_CONFIGURATOR="${JAVA_LAYERED_GRAPH_CONFIGURATOR:-external/elk/plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/GraphConfigurator.java}"
RUST_LAYERED_GRAPH_CONFIGURATOR="${RUST_LAYERED_GRAPH_CONFIGURATOR:-plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph_configurator.rs}"
REPORT_FILE="${1:-tests/layered_phase_wiring_parity.md}"
ARTIFACT_DIR="${LAYERED_PHASE_WIRING_PARITY_ARTIFACT_DIR:-tests/layered_phase_wiring}"
STRICT_MODE="${LAYERED_PHASE_WIRING_PARITY_STRICT:-false}"

set -- \
  --java-file "$JAVA_LAYERED_GRAPH_CONFIGURATOR" \
  --rust-file "$RUST_LAYERED_GRAPH_CONFIGURATOR" \
  --report-file "$REPORT_FILE" \
  --artifact-dir "$ARTIFACT_DIR"

if [ "$STRICT_MODE" = "true" ]; then
  set -- "$@" --strict
fi

python3 "$SCRIPT_DIR/check_layered_phase_wiring_parity.py" "$@"
