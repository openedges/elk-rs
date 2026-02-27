# ELK (Java) vs elk-rs Functional Parity

## Overview

elk-rs is a Rust port of [Eclipse Layout Kernel (ELK)](https://www.eclipse.org/elk/),
a Java graph layout library. The goal is **layout-identical output**: given the
same input graph and options, elk-rs must produce the same node coordinates, edge
routes, and label positions as Java ELK.

This document describes the parity verification system: what is checked, how it
is checked, the current status, known exceptions, and directory conventions.

## Gate Execution Order

Local and CI share the same sequence:

1. Phase wiring parity (static structure)
2. `cargo build --workspace`
3. `cargo clippy --workspace --all-targets`
4. `cargo test --workspace`
5. Model parity (layout output comparison, release/regression only)

### Pass Criteria

| Step | Gate | Criterion |
|------|------|-----------|
| 1 | Phase wiring | `parity/layered_phase_wiring_parity.md` reports `status: ok` |
| 2 | Build | Zero errors and warnings |
| 3 | Clippy | Zero warnings |
| 4 | Unit tests | Zero failures |
| 5 | Model parity | Drift count within known exceptions |

### Failure Analysis Loop

1. Reproduce the failure in isolation (single crate/test).
2. Narrow scope to the relevant crate or processor.
3. If needed, compare Java/Rust phase traces to identify the divergence step.
4. Record root cause, hypothesis, and reproduction command in `HISTORY.md`.

## Architecture

```
Java ELK (reference)          elk-rs (implementation under test)
  |                              |
  v                              v
ElkModelParityExportTest      model_parity_layout_runner
  |  (Tycho/OSGi test)          |  (cargo binary)
  v                              v
java/input/*.json ----+----> rust/layout/*.json
java/layout/*.json    |
                      v
        compare_model_parity_layouts.py
                      |
                      v
              report.md + diff_details.tsv
```

Both sides consume the same input JSON (serialized ELK graphs from
`external/elk-models`). The comparison tool diffs the layout outputs
field-by-field with configurable numeric tolerance (`--abs-tol`, default 1e-6).

## Verification Layers

### 1. Code Quality

Basic build, lint, and test health.

```sh
cargo build --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
```

**Current status** (2026-02-26): 509 tests, 0 failures.

### 2. Model Parity (Layout Output Comparison)

The primary parity gate. Compares complete layout output of 1448 models
(examples, tests, tickets, realworld) between Java ELK and elk-rs.

```sh
# Full run: Java export + Rust layout + comparison
JAVA_PARITY_REQUIRE_CLEAN_EXTERNAL_ELK=false \
  sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models parity/model_parity

# Skip Java export (reuse existing Java baseline)
MODEL_PARITY_SKIP_JAVA_EXPORT=true \
  sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models parity/model_parity
```

**Current status** (2026-02-26):
- Total: 1448 models, Compared: 1438, **Matched: 1438**, Drift: 0, Skipped: 10
- Match rate: **100%**
- Skipped: 9 Java exceptions + 1 Java NaN bug (`213_componentsCompaction.elkt`)

Output reports:
- `parity/model_parity/report.md` -- summary with drift classification
- `parity/model_parity/diff_details.tsv` -- per-model detail rows
- `parity/model_parity/rust_manifest.tsv` -- Rust runner results

### 3. Phase-Step Verification (Layered Pipeline Trace)

Compares intermediate state after each layered pipeline step (50+ processors).
Detects at which processing step divergence first occurs, enabling targeted
debugging.

```sh
# Generate Java traces:
sh scripts/run_java_phase_trace.sh <input_dir> <output_dir>

# Generate Rust traces:
cargo run --release -p org-eclipse-elk-graph-json --bin model_parity_layout_runner \
  -- --trace-dir <output_dir> <input.json>

# Compare traces:
python3 scripts/compare_phase_traces.py <java_trace_dir> <rust_trace_dir> --batch

# Summarize gate:
python3 scripts/summarize_phase_gate.py \
  --java-manifest ... --rust-manifest ... \
  --java-trace-dir ... --rust-trace-dir ... \
  --compare-json ... --output-md parity/model_parity/phase_gate_latest.md
```

**Current status**: gate_pass=**true**, 1439/1439 models match at all 50+ steps.

Output: `parity/model_parity/phase_gate_latest.md`

### 4. API/Metadata Parity

Verifies that algorithm registrations, option definitions, and feature
declarations match between Java and Rust:

| Check | Script | Status |
|-------|--------|--------|
| Algorithm IDs | `check_algorithm_id_parity.sh` | ok |
| Algorithm names | `check_algorithm_name_parity.sh` | ok |
| Algorithm descriptions | `check_algorithm_description_parity.sh` | ok |
| Algorithm categories | `check_algorithm_category_parity.sh` | ok |
| Algorithm metadata | `check_algorithm_metadata_parity.sh` | ok |
| Algorithm features | `check_algorithm_feature_parity.sh` | ok |
| Option support | `check_algorithm_option_support_parity.sh` | ok |
| Option defaults | `check_algorithm_option_default_parity.sh` | ok |
| Option default values | `check_algorithm_option_default_value_parity.sh` | ok |
| Core option deps | `check_core_option_dependency_parity.sh` | ok |
| Core options | `check_core_options_parity.sh` | ok |
| Phase wiring | `check_layered_phase_wiring_parity.sh` | ok |
| Test method counts | `check_layered_issue_test_parity.sh` | ok |
| Test module parity | `check_java_test_module_parity.sh` | ok |

Reports are written to `parity/*.md`.

### 5. Test Parity (Structural Coverage)

Checks that Java test modules and issue test methods have been ported to Rust
at the structural level (file/method count mapping).

```sh
sh scripts/check_layered_issue_test_parity.sh
sh scripts/check_java_test_module_parity.sh
```

Output: `parity/layered_issue_test_parity.md`, `parity/java_test_module_parity.md`

Note: This checks structure and count, not semantic equivalence of test logic.

### 6. Performance and Regression Gates

Prevents performance regression against Rust baselines and monitors Java
comparison.

```sh
# Rust baseline regression check:
PARITY_COMPARE_MODE=baseline sh scripts/check_parity_regression.sh 5 3

# Runtime budget check:
sh scripts/check_recursive_parity_runtime_budget.sh

# Java parity:
sh scripts/check_java_parity.sh
sh scripts/check_java_parity_scenarios.sh
```

## Known Drift (0 models)

All comparable models match. One model (`213_componentsCompaction.elkt`) is excluded
from comparison due to a Java ELK bug (see below).

### Resolved Drift History

| Model | Resolution |
|-------|------------|
| `213_componentsCompaction.elkt` | Excluded — Java `ComponentsCompactor` produces NaN y-coordinates (see Skipped Models) |

### Resolved Drift (2026-02-26)

| Model | Fix |
|-------|-----|
| `next_to_port_if_possible_inside.elkt` | Removed `.max(0.0)` clamp in `components_processor.rs` `combine_component_group` |
| `multilabels_compound.elkt` | Same fix — negative graph sizes are valid intermediate values (matching Java) |

## Exception Cases

### Java Non-Determinism (Opposing Self-Loop Routing)

**Problem**: `SelfHyperLoop.computePortsPerSide()` uses `ArrayListMultimap`
(HashMap-backed) whose `keySet()` iteration order varies across JVM invocations.
When opposing self-loop routing hits a tie, ~80 models flip between runs.

**Fix**: Java patch replaces `ArrayListMultimap` with
`MultimapBuilder.enumKeys()` for deterministic enum-ordinal iteration. Rust uses
matching clockwise sort order in `opposing_side_order_rank`.

- Java patch: `scripts/java/patches/0001-deterministic-opposing-self-loop-routing.patch`
- Applied automatically during parity export (set `JAVA_PARITY_APPLY_PATCHES=false` to skip)
- See `scripts/java/patches/README.md` for patch management

### Element Reference Hash Randomness

**Problem**: Java ELK generates element IDs using `System.identityHashCode()`,
producing different hash suffixes (`P1_g337798` vs `P1_g895123`) each run.
These appear in `id`, `incomingShape`, `outgoingShape`, `sources`, `targets`,
and `container` JSON fields.

**Impact**: Does NOT affect parity comparison (Java->Rust uses shared input IDs).
Only affects Java->Java determinism checks.

**Handling**: `compare_model_parity_layouts.py --skip-fields` option filters
reference fields during comparison:
```sh
python3 scripts/compare_model_parity_layouts.py \
  --manifest ... --report ... --details ... \
  --skip-fields 'id,incomingShape,outgoingShape,sources,targets,container'
```

**Verified**: 1439/1439 models produce identical coordinates across Java runs
(0 coordinate diffs when reference fields are excluded).

### Inside Self-Loop Node Compaction

Rust's `json_importer.rs` forces childless inside self-loop nodes to `width=4.0`
(matching Java behavior). A `has_no_children` guard ensures nodes with children
retain their correct width from recursive layout.

### Skipped Models (10)

Models where Java ELK itself reports a non-ok status (exception, timeout, NaN
output) are excluded from comparison. These are tracked in the manifest as
`java_status != ok`.

| Count | Category | Description |
|------:|----------|-------------|
| 9 | exception/timeout | Java ELK throws or times out during layout |
| 1 | nan_output | `213_componentsCompaction.elkt` — Java `ComponentsCompactor` produces NaN y-coordinates (73 fields) due to degenerate `∞ - ∞` bounding box computation with zero-size nodes. Rust output is correct. |

## CI Integration

| Workflow | Gates |
|----------|-------|
| `.github/workflows/ci.yml` | Fast checks (`run_fast_checks.sh`): build, clippy, test, wiring parity |
| `.github/workflows/parity.yml` | Full performance/parity: algorithm/core parity, phase wiring, report artifacts |

## Operational Principles

- Run `RELEASE_CHECKLIST.md` before releases.
- Record parity metrics, experiment logs, and exception reasons in `HISTORY.md`.
- Follow the `Directory policy (keep vs temporary)` in `parity/README.md`.
- See `parity/java_parity_triage.md` for Java parity failure triage procedures.

## Directory Structure

```
parity/
  README.md                          # Parity workflow reference (scripts, CSVs)
  PARITY.md                         # This document

  model_parity/                      # Compact parity snapshot (tracked)
    report.md                        #   Summary report (KEEP)
    diff_details.tsv                 #   Per-model diffs (KEEP)
    rust_manifest.tsv                #   Rust runner manifest (KEEP)
    java/java_manifest.tsv           #   Java export manifest (KEEP)
    phase_gate_latest.md             #   Phase-step gate result (KEEP)
    java/input/   (gitignored)       #   Java input JSON (TEMP)
    java/layout/  (gitignored)       #   Java layout JSON (TEMP)
    rust/layout/  (gitignored)       #   Rust layout JSON (TEMP)
    rust_runner_progress.tsv         #   Progress log (TEMP, gitignored)
    drift_summary*.txt               #   Diagnostic outputs (TEMP, gitignored)

  model_parity_full/                 # Full parity run (all gitignored)
    report.md                        #   Summary report
    diff_details.tsv                 #   Per-model diffs
    rust_manifest.tsv                #   Rust runner manifest
    java/java_manifest.tsv           #   Java export manifest
    phase_root_summary.md            #   Phase root analysis
    phase_focus_top.md               #   Top drift focus queue
    java/input/, java/layout/        #   Java JSON payloads
    rust/layout/                     #   Rust JSON payloads

  algorithm_*_parity.md              # API/metadata parity reports
  layered_phase_wiring_parity.md     # Phase wiring check
  layered_issue_test_parity.md       # Test method count parity
  java_test_module_parity.md         # Module-level test parity

scripts/
  run_model_parity_elk_vs_rust.sh    # Full parity pipeline (Java+Rust+compare)
  run_java_model_parity_export.sh    # Java-only export with patch support
  compare_model_parity_layouts.py    # JSON diff tool (--skip-fields, --strict)
  check_*.sh                         # Individual parity gate checks
  java/patches/                      # Java determinism patches
    0001-deterministic-opposing-self-loop-routing.patch
    README.md
```

### File Lifecycle

| Category | Location | Git Status | Lifecycle |
|----------|----------|------------|-----------|
| Compact summaries | `parity/model_parity/*.{md,tsv}` | Tracked | Updated on each parity run, committed |
| JSON payloads | `parity/model_parity/**/layout/` | Gitignored | Regenerated on each run, auto-cleaned |
| Full run outputs | `parity/model_parity_full/` | Gitignored | Ephemeral, entire dir is temp |
| API parity reports | `parity/*.md` | Tracked | Updated by check scripts, committed |
| Java patches | `scripts/java/patches/` | Tracked | Manual maintenance |

### Cleanup

```sh
# Remove all gitignored temp files
sh scripts/clean_parity_temp.sh --apply

# Include tracked runtime files too
sh scripts/clean_parity_temp.sh --apply --include-tracked
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `JAVA_PARITY_APPLY_PATCHES` | `true` | Apply patches from `scripts/java/patches/` |
| `JAVA_PARITY_PATCHES_DIR` | `$REPO_ROOT/scripts/java/patches` | Patch directory |
| `JAVA_PARITY_REQUIRE_CLEAN_EXTERNAL_ELK` | `true` | Require clean `external/elk` tree |
| `JAVA_PARITY_EXTERNAL_ISOLATE` | `true` | Use isolation worktree for Java build |
| `MODEL_PARITY_SKIP_JAVA_EXPORT` | `false` | Reuse existing Java baseline |
| `MODEL_PARITY_RANDOM_SEED` | `1` | Random seed for both sides |
| `MODEL_PARITY_STRICT` | `false` | Exit non-zero on drift |
| `MODEL_PARITY_ABS_TOL` | `1e-6` | Numeric comparison tolerance |
