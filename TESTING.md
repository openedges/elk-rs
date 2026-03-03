# Testing and Validation Guide

Single entry point for the elk-rs validation flow. Covers environment setup,
verification items, scenario-based procedures, release checklist, and known
issues.

For detailed parity system documentation (architecture, environment variables,
directory structure, exception cases), see [`tests/PARITY.md`](tests/PARITY.md).

---

## 1. Environment Setup

### Required Tools

| Tool | Minimum Version | Purpose |
|------|-----------------|---------|
| Rust toolchain | stable | Build, test, clippy |
| Java JDK | 17+ | Java baseline export |
| Maven | 3.6+ | ELK build (path configurable via `JAVA_PARITY_MVN_BIN`) |
| Python | 3.8+ | `compare_model_parity_layouts.py`, `compare_phase_traces.py`, etc. |
| Node.js | 16+ | JS tests, parity tests |
| wasm-pack | latest | WASM build (`wasm32-unknown-unknown` target required) |

### Submodule Initialization

```sh
git submodule update --init --recursive
# Verify all 3 submodules: external/elk, external/elkjs, external/elk-models
ls external/elk external/elkjs external/elk-models
```

### Smoke Test

```sh
cargo build --workspace && cargo test --workspace
```

### First Java Baseline Run

```sh
sh scripts/java_model_parity_trace.sh external/elk-models tests/model_parity
# Includes Maven + Tycho build; first run takes ~10 minutes
```

### Java Determinism Patches

Java ELK has non-deterministic code paths (HashMap iteration order) that produce
different layout results across JVM invocations. Without patches, ~80 models flip
between runs, causing spurious parity drift.

**Root cause**: `SelfHyperLoop.computePortsPerSide()` uses `ArrayListMultimap`
(HashMap-backed) whose `keySet()` iteration order depends on identity hash codes.
When opposing self-loop routing hits a tie, the result varies per JVM run.

**Fix**: A patch in `scripts/java/patches/` replaces `ArrayListMultimap` with
`MultimapBuilder.enumKeys()` for deterministic enum-ordinal iteration. Rust uses
a matching clockwise sort order in `opposing_side_order_rank`.

Patches are applied automatically during Java baseline export:

- Applied to an **isolation worktree** (never modifies `external/elk` directly)
- Controlled by `JAVA_PARITY_APPLY_PATCHES=true` (default)
- Set `JAVA_PARITY_APPLY_PATCHES=false` to skip
- See `scripts/java/patches/README.md` for patch inventory and how to add new patches

---

## 2. Verification Items

### A. Code Quality (3 items)

| Item | Command | Pass Criteria |
|------|---------|---------------|
| Build | `cargo build --workspace` | Zero errors/warnings |
| Lint | `cargo clippy --workspace --all-targets` | Zero warnings |
| Unit tests | `cargo test --workspace` | Zero failures (currently 653 tests) |

### B. Layout Equivalence (3 items)

| Item | Comparison | Current Status |
|------|------------|----------------|
| Model parity | Java vs Rust layout JSON | 1438/1438 = 100% |
| Phase-step trace | Intermediate state across 50+ processors | 1439/1439 = 100% |
| JS parity | 3-way: elk-rs JS vs elkjs vs Java | 550/550 = 100% |

Commands and outputs:

```sh
# Model parity
MODEL_PARITY_SKIP_JAVA_EXPORT=true \
  sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models tests/model_parity
# Output: tests/model_parity/report.md, tests/model_parity/diff_details.tsv

# Phase-step trace
python3 scripts/compare_phase_traces.py <java_trace_dir> <rust_trace_dir> --batch
# Output: tests/model_parity/phase_gate_latest.md

# JS parity
cd plugins/org.eclipse.elk.js && npm run test:parity
# Output: test/parity/results/parity-report.json
```

### C. API/Metadata Equivalence (14 items)

| Item | Script | Output |
|------|--------|--------|
| Algorithm ID | `check_algorithm_id_parity.sh` | `tests/algorithm_id_parity.md` |
| Algorithm name | `check_algorithm_name_parity.sh` | `tests/algorithm_name_parity.md` |
| Algorithm description | `check_algorithm_description_parity.sh` | `tests/algorithm_description_parity.md` |
| Algorithm category | `check_algorithm_category_parity.sh` | `tests/algorithm_category_parity.md` |
| Algorithm metadata | `check_algorithm_metadata_parity.sh` | `tests/algorithm_metadata_parity.md` |
| Algorithm feature | `check_algorithm_feature_parity.sh` | `tests/algorithm_feature_parity.md` |
| Option support | `check_algorithm_option_support_parity.sh` | `tests/algorithm_option_support_parity.md` |
| Option default | `check_algorithm_option_default_parity.sh` | `tests/algorithm_option_default_parity.md` |
| Option default value | `check_algorithm_option_default_value_parity.sh` | `tests/algorithm_option_default_value_parity.md` |
| Core options | `check_core_options_parity.sh` | `tests/core_options_parity.md` |
| Core option dependency | `check_core_option_dependency_parity.sh` | `tests/core_option_dependency_parity.md` |
| Phase wiring | `check_layered_phase_wiring_parity.sh` | `tests/layered_phase_wiring_parity.md` |
| Test method count | `check_layered_issue_test_parity.sh` | `tests/layered_issue_test_parity.md` |
| Test module parity | `check_java_test_module_parity.sh` | `tests/java_test_module_parity.md` |

Strict mode must be enabled for releases (e.g., `ALGORITHM_ID_PARITY_STRICT=true`).

### D. Performance (3 items)

| Item | Command | Description |
|------|---------|-------------|
| Rust baseline regression | `PARITY_COMPARE_MODE=baseline sh scripts/check_parity_regression.sh 5 3` | Checks if current Rust is >5% slower than baseline |
| Runtime budget | `sh scripts/check_recursive_parity_runtime_budget.sh` | Recursive layout time budget |
| Java comparison | `sh scripts/check_java_parity.sh` | Performance comparison against Java |

See `scripts/README.md` for default arguments and environment knobs.

### E. Performance Benchmark — 5-Way Comparison (5 items)

| Item | Command | Description |
|------|---------|-------------|
| Synthetic 5-way | `sh scripts/run_perf_benchmark.sh synthetic` | All 5 engines on synthetic scenarios |
| Model 5-way | `sh scripts/run_perf_benchmark.sh models` | All 5 engines on model JSON inputs |
| JS benchmark | `cd plugins/org.eclipse.elk.js && npm run bench` | elkjs/NAPI/WASM only |
| Comparison report | `python3 scripts/compare_perf_results.py tests/perf/` | Markdown report generation |
| Report output | `tests/perf/report.md` | Per-engine and per-scenario comparison |

Engines: Java (`RecursiveGraphLayoutEngine` via JSON), Rust native (direct `ElkNode`),
Rust API (`layout_json`), NAPI (Node.js native addon), WASM, elkjs (GWT-compiled).

Prerequisite: NAPI/WASM builds must be complete (`cd plugins/org.eclipse.elk.js && sh build.sh`).

---

## 3. Scenario-Based Procedures

### 3.1 Daily Development (after code changes)

```sh
cargo build --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
```

### 3.2 Processor/Feature Addition

All of 3.1 + phase wiring parity + model parity:

```sh
# After running 3.1:
LAYERED_PHASE_WIRING_PARITY_STRICT=true sh scripts/check_layered_phase_wiring_parity.sh
MODEL_PARITY_SKIP_JAVA_EXPORT=true \
  sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models tests/model_parity
```

### 3.3 Option/Metadata Changes

All of 3.1 + API/metadata parity (9 algorithm + 2 core checks):

```sh
# After running 3.1:
sh scripts/check_core_options_parity.sh
sh scripts/check_core_option_dependency_parity.sh
ALGORITHM_ID_PARITY_STRICT=true sh scripts/check_algorithm_id_parity.sh
ALGORITHM_CATEGORY_PARITY_STRICT=true sh scripts/check_algorithm_category_parity.sh
ALGORITHM_NAME_PARITY_STRICT=true sh scripts/check_algorithm_name_parity.sh
ALGORITHM_DESCRIPTION_PARITY_STRICT=true sh scripts/check_algorithm_description_parity.sh
ALGORITHM_OPTION_SUPPORT_PARITY_STRICT=true sh scripts/check_algorithm_option_support_parity.sh
ALGORITHM_OPTION_DEFAULT_PARITY_STRICT=true sh scripts/check_algorithm_option_default_parity.sh
ALGORITHM_OPTION_DEFAULT_VALUE_PARITY_STRICT=true sh scripts/check_algorithm_option_default_value_parity.sh
ALGORITHM_FEATURE_PARITY_STRICT=true sh scripts/check_algorithm_feature_parity.sh
ALGORITHM_METADATA_PARITY_STRICT=true sh scripts/check_algorithm_metadata_parity.sh
```

### 3.4 JS/WASM Changes

All of 3.1 + JS tests + JS parity:

```sh
# After running 3.1:
cd plugins/org.eclipse.elk.js
npm test
npm run test:parity
```

### 3.5 Before Submitting a PR

Run all applicable items from 3.1–3.4 based on the type of changes.

### 3.6 Before Release (Release Checklist)

#### Validation Flow (run in order)

```sh
# 1. Phase wiring parity
LAYERED_PHASE_WIRING_PARITY_STRICT=true sh scripts/check_layered_phase_wiring_parity.sh

# 2. Build
cargo build --workspace

# 3. Clippy
cargo clippy --workspace --all-targets

# 4. Unit tests
cargo test --workspace

# 5. Full model parity (layout output comparison)
MODEL_PARITY_SKIP_JAVA_EXPORT=true \
  sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models tests/model_parity_full

# 6. Phase-step trace (intermediate state across 50+ processors)
python3 scripts/compare_phase_traces.py \
  tests/model_parity/java_trace tests/model_parity/rust_trace --batch

# 7. Release build
cargo build --workspace --release
```

Pass criteria:

- Step 1: `tests/layered_phase_wiring_parity.md` reports `status: ok`
- Step 2: Zero build errors/warnings
- Step 3: Zero clippy warnings
- Step 4: Zero test failures
- Step 5: Model parity report generated, drift=0 (or within known exceptions)
- Step 6: Phase-step trace report generated, all steps match (or equivalent intermediate only)
- Step 7: Release profile build succeeds

**Note**: Steps 5 and 6 are both required to confirm full equivalence. Model parity verifies
final layout output; phase-step trace verifies intermediate state after each of the 50+
layered processors. A model may produce identical final output while diverging at an
intermediate step (equivalent intermediate), so both checks are necessary.

Failure triage:

1. Reproduce with a focused command (single crate/test/script).
2. Narrow scope to the failing module/phase.
3. If needed, compare Java/Rust phase traces to pinpoint the divergence.
4. Record root-cause hypothesis and reproduction command in `HISTORY.md`.

#### Metadata Gate (strict mode, required)

Run all commands from § 3.3 plus phase wiring:

```sh
LAYERED_PHASE_WIRING_PARITY_STRICT=true sh scripts/check_layered_phase_wiring_parity.sh
```

If any report (`tests/*_parity.md`) does not report `status: ok`, stop the release.

#### Performance Gate (required + recommended)

Required:

```sh
PARITY_COMPARE_MODE=baseline sh scripts/check_parity_regression.sh 5 3
sh scripts/check_recursive_parity_runtime_budget.sh \
  tests/results_recursive_layout_scenarios.csv default tests/recursive_runtime_budget.md
```

Java comparison (recommended):

```sh
sh scripts/check_java_parity.sh \
  tests/results_layered_issue_scenarios.csv tests/java_results_layered_issue_scenarios.csv 3 0
sh scripts/check_java_parity_scenarios.sh \
  tests/results_layered_issue_scenarios.csv tests/java_results_layered_issue_scenarios.csv 3 \
  tests/java_parity_thresholds.csv
```

#### Release Decision Rules

- `code quality + parity + runtime budget` all pass: **release allowed**.
- Only `baseline 5% regression` fails and Java parity passes:
  - Default policy: investigate the regression before release.
  - Exception: emergency release is allowed, but document the parity risk in release notes.
- Java parity fails: **stop the release**.

#### Artifacts to Review

The following reports are generated during release validation:

- `tests/core_options_parity.md`
- `tests/core_option_dependency_parity.md`
- `tests/algorithm_*_parity.md`
- `tests/layered_phase_wiring_parity.md`
- `tests/recursive_runtime_budget.md`
- `tests/java_vs_rust.md`
- `tests/java_vs_rust_baseline.md`
- `tests/java_parity_status.md`

#### Baseline Update Procedure

Update baselines only when all conditions are met:
- There is an intentional performance change (algorithm/options/environment).
- Repeated measurements show stable variance.
- The team approves the baseline movement.

```sh
sh scripts/run_parity_layered_issue_scenarios.sh \
  "issue_405,issue_603,issue_680,issue_871,issue_905" 20 3 \
  tests/results_layered_issue_scenarios.csv
PARITY_RECURSIVE_SCENARIO_PROFILE=default sh scripts/run_parity_recursive_layout_scenarios.sh \
  "" 5 1 tests/results_recursive_layout_scenarios.csv
sh scripts/update_parity_baseline.sh
sh scripts/update_parity_recursive_scenarios_baseline.sh
PARITY_COMPARE_MODE=baseline sh scripts/check_parity_regression.sh 5 3
```

For detailed baseline operations, see `tests/baselines/POLICY.md`.

### 3.7 Performance Benchmark (5-Way Comparison)

Prerequisite: NAPI and WASM builds must be complete.

```sh
cd plugins/org.eclipse.elk.js && sh build.sh && cd -
```

Quick synthetic benchmark (5 scenarios, no Java):

```sh
PERF_SKIP_JAVA=true sh scripts/run_perf_benchmark.sh synthetic 5 1 tests/perf
python3 scripts/compare_perf_results.py tests/perf/
```

Full model benchmark (all engines including Java):

```sh
sh scripts/run_perf_benchmark.sh models 20 3 tests/perf
```

JS-only benchmark:

```sh
cd plugins/org.eclipse.elk.js && npm run bench
```

### 3.8 Porting a New ELK Version

See the porting workflow in `VERSIONING.md`.

---

## 4. Known Issues

### 4.1 Accepted Exceptions

| Category | Item | Count | Description |
|----------|------|-------|-------------|
| Java non-ok | exception/timeout | 9 | Models where Java ELK itself throws or times out |
| Java non-ok | NaN output | 1 | `213_componentsCompaction.elkt` — Java `ComponentsCompactor` NaN propagation; Rust output is correct |
| ELKJS drift | GWT artifact | 20 | elk-rs matches Java 100%; only elkjs diverges (y-offset, floating-point) |

### 4.2 Resolved: `elk_live_examples_test` (2026-03-03)

Previously failed due to cross-hierarchy edges causing `UnsupportedGraphException` panics
and potential hangs. Fixed by:
1. Converting `transform_edge()` panics to graceful edge skips (matching Java's exception-and-skip semantics)
2. Adding per-example 30s timeout via thread+channel pattern to prevent test hangs

The test now passes all 45 `.elkt` examples. This does not affect model parity (which uses JSON input).

### 4.3 Stale Maven Cache (Java Export)

**Symptom**: Java determinism patches apply correctly (verified in source tree)
but the parity test still produces unpatched output.

**Root cause**: Stale ELK SNAPSHOT JARs in `~/.m2/repository/org/eclipse/elk/`.
Tycho/OSGi resolves bundles by highest version, so a leftover `X.Y.Z-SNAPSHOT`
that is higher than the version being built silently overrides the freshly
compiled patched JARs at test runtime.

**Prevention**: `java_model_parity_trace.sh` automatically purges all ELK
SNAPSHOT directories from the Maven cache before building. This guard runs
unconditionally (unless `JAVA_PARITY_DRY_RUN=true`).

**Manual fix**:

```sh
find ~/.m2/repository/org/eclipse/elk -name '*-SNAPSHOT' -type d -exec rm -rf {} +
```

Then re-run without `MODEL_PARITY_SKIP_JAVA_EXPORT=true` to regenerate the
Java baseline.

**Diagnosis**: Use `javap -c` on the suspect class inside the cached JAR to
confirm which code variant is present:

```sh
# Check if the patched MultimapBuilder is in the built JAR:
jar_path=$(find ~/.m2/repository/org/eclipse/elk -name '*.jar' -path '*/alg.layered/*' | head -1)
javap -c -classpath "$jar_path" org.eclipse.elk.alg.layered.intermediate.loops.SelfHyperLoop \
  | grep -E 'MultimapBuilder|ArrayListMultimap'
```

### 4.4 Failure Analysis

See the "Failure Analysis Loop" section in `tests/PARITY.md`.

---

## 5. Documentation Map

| Purpose | Document |
|---------|----------|
| Full verification system details (architecture, env vars, directory) | `tests/PARITY.md` |
| Script reference | `tests/README.md`, `scripts/README.md` |
| Baseline management | `tests/baselines/POLICY.md` |
| Java CI failure triage | `tests/java_parity_triage.md` |
| Versioning and porting policy | `VERSIONING.md` |
| Project status | `AGENTS.md` |
| Development history | `HISTORY.md` |
| Java determinism patches | `scripts/java/patches/README.md` |
