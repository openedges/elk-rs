# ELK (Java) vs elk-rs Functional Parity

## Overview

elk-rs is a Rust port of [Eclipse Layout Kernel (ELK)](https://www.eclipse.org/elk/),
a Java graph layout library. The goal is **layout-identical output**: given the
same input graph and options, elk-rs must produce the same node coordinates, edge
routes, and label positions as Java ELK.

This document describes the parity verification system: what is checked, how it
is checked, the current status, known exceptions, and directory conventions.

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

### 1. Unit Tests (`cargo test`)

Standard Rust unit and integration tests. Each plugin crate
(`org-eclipse-elk-core`, `org-eclipse-elk-alg-layered`, etc.) has its own test
suite covering individual algorithms, data structures, and edge cases.

```sh
cargo test --workspace
```

### 2. Model Parity (Layout Output Comparison)

The primary parity gate. Compares complete layout output of 1448 models
(examples, tests, tickets, realworld) between Java ELK and elk-rs.

```sh
# Full run: Java export + Rust layout + comparison
JAVA_PARITY_REQUIRE_CLEAN_EXTERNAL_ELK=false \
  sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models perf/model_parity

# Skip Java export (reuse existing Java baseline)
MODEL_PARITY_SKIP_JAVA_EXPORT=true \
  sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models perf/model_parity
```

**Current status** (2026-02-26):
- Total: 1448 models, Compared: 1439, **Matched: 1436**, Drift: 3, Skipped: 9
- Match rate: **99.8%**

Output reports:
- `perf/model_parity/report.md` — summary with drift classification
- `perf/model_parity/diff_details.tsv` — per-model detail rows
- `perf/model_parity/rust_manifest.tsv` — Rust runner results

### 3. Phase-Step Verification (Layered Pipeline Trace)

Compares intermediate state after each layered pipeline step (50+ processors).
Detects at which processing step divergence first occurs, enabling targeted
debugging.

```sh
# Phase gate check (requires trace exports from both sides)
python3 scripts/check_layered_phase_wiring_parity.py
```

**Current status**: gate_pass=**true**, 1439/1439 models match at all 50+ steps.

Output: `perf/model_parity/phase_gate_latest.md`

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

Reports are written to `perf/*.md`.

## Known Drift (3 models)

| Model | Diffs | Root Cause |
|-------|------:|------------|
| `next_to_port_if_possible_inside.elkt` | 5 | Stale Java reference; port label inside-cell layout not yet refreshed |
| `multilabels_compound.elkt` | 6 | Phase 4 (node placement) coordinate; compound label layout edge case |
| `213_componentsCompaction.elkt` | 20 | 1D compaction not fully ported; post-processing differences |

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

**Impact**: Does NOT affect parity comparison (Java→Rust uses shared input IDs).
Only affects Java→Java determinism checks.

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

### Skipped Models (9)

Models where Java ELK itself reports a non-ok status (exception, timeout) are
excluded from comparison. These are tracked in the manifest as
`java_status != ok`.

## Directory Structure

```
perf/
  README.md                          # Perf workflow reference (scripts, CSVs)
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
| Compact summaries | `perf/model_parity/*.{md,tsv}` | Tracked | Updated on each parity run, committed |
| JSON payloads | `perf/model_parity/**/layout/` | Gitignored | Regenerated on each run, auto-cleaned |
| Full run outputs | `perf/model_parity_full/` | Gitignored | Ephemeral, entire dir is temp |
| API parity reports | `perf/*.md` | Tracked | Updated by check scripts, committed |
| Java patches | `scripts/java/patches/` | Tracked | Manual maintenance |

### Cleanup

```sh
# Remove all gitignored temp files
sh scripts/clean_perf_temp.sh --apply

# Include tracked runtime files too
sh scripts/clean_perf_temp.sh --apply --include-tracked
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
