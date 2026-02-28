Scripts overview:

Release readiness quick run:
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo build --workspace --release`
- `PARITY_COMPARE_MODE=baseline sh scripts/check_parity_regression.sh 5 3`
- `sh scripts/check_recursive_parity_runtime_budget.sh tests/results_recursive_layout_scenarios.csv default tests/recursive_runtime_budget.md`
- `sh scripts/check_java_parity.sh tests/results_layered_issue_scenarios.csv tests/java_results_layered_issue_scenarios.csv 3 0`
- `sh scripts/check_java_parity_scenarios.sh tests/results_layered_issue_scenarios.csv tests/java_results_layered_issue_scenarios.csv 3 tests/java_parity_thresholds.csv`
- Final release criteria and go/no-go rules are documented in `TESTING.md` § 3.6.

- `run_parity_comment_attachment.sh [count] [iterations] [warmup] [output]`
- `run_parity_graph_validation.sh [nodes] [edges] [iterations] [warmup] [mode] [output]`
- `run_parity_recursive_layout.sh [nodes] [edges] [iterations] [warmup] [algorithm] [validate_graph] [validate_options] [output]`
- `run_parity_layered_layout.sh [nodes] [edges] [iterations] [warmup] [validate_graph] [validate_options] [output]`
- `run_parity_recursive_layout_scenarios.sh [scenarios] [iterations] [warmup] [output]` (`fixed_dense`, `fixed_sparse`, `random_dense`, `random_sparse`, `box_sparse`, `box_large`, `fixed_validated`, `random_validated`, `box_validated` preset scenarios; when `scenarios` is empty, the default set is selected by `PARITY_RECURSIVE_SCENARIO_PROFILE=quick|default|full`)
- `run_parity_layered_issue_scenarios.sh [scenarios] [iterations] [warmup] [output]`
- `run_java_parity_layered_issue_scenarios.sh [scenarios] [iterations] [warmup] [output]` (runs the external ELK Java layered benchmark test; benchmark test source is temporarily injected from a repository template and cleaned up automatically)
- `run_java_model_parity_export.sh [models_root] [output_dir]` (injects `scripts/java/ElkModelParityExportTest.java` into external ELK tests and exports model-level Java input/layout JSON + manifest for parity comparison)
- `run_model_parity_elk_vs_rust.sh [models_root] [output_root]` (runs Java export -> Rust layout replay -> JSON diff report pipeline for `external/elk-models`)
- `compare_model_parity_layouts.py --manifest <rust_manifest.tsv> --report <report.md> --details <details.tsv>` (numeric-tolerant structural comparison of Java vs Rust layout JSON results)
- `run_java_phase_trace.sh [models_root] [output_dir]` (exports Java layered phase snapshots as `step_*.json`; supports JSON input roots such as `tests/model_parity/java/input`)
- `compare_phase_traces.py <java_trace_dir> <rust_trace_dir> --batch --json` (phase-by-phase Java/Rust trace diff with first-divergence detection)
- `summarize_phase_gate.py --java-manifest <java_manifest.tsv> --rust-manifest <rust_manifest.tsv> --java-trace-dir <dir> --rust-trace-dir <dir> --compare-json <phase_compare.json> --output-json <out.json> --output-md <out.md>` (applies phase-gate rules: `java_status=ok` baseline, 비교불가=error, 최초 실패 step만 error 집계)
- `run_parity_all.sh` (runs all parity scripts with defaults; supports env overrides)
- `compare_parity_results.sh [window]` (`PARITY_COMPARE_MODE=window|baseline|both`, default window; baseline mode compares against `PARITY_BASELINE_LAYERED_FILE` + `PARITY_BASELINE_RECURSIVE_SCENARIOS_FILE`; scenario files auto-filter current-side rows to each scenario's latest run config tuple to avoid mixed-window contamination)
- `check_recursive_parity_runtime_budget.sh [results_file] [profile] [report]` (checks whether latest per-scenario `avg_iteration_ms` in recursive scenario CSV exceeds profile budgets (`quick|default|full`); default budgets are `RECURSIVE_BUDGET_MS_QUICK=40`, `RECURSIVE_BUDGET_MS_DEFAULT=60`, `RECURSIVE_BUDGET_MS_FULL=120`; with `RECURSIVE_RUNTIME_BUDGET_STRICT=true`, budget violations fail the run)
- `summarize_parity_results.sh [output]` (writes `tests/summary.md` by default)
- `check_parity_regression.sh [threshold] [window]` (`PARITY_COMPARE_MODE=window|baseline|both`; baseline mode evaluates regressions against `PARITY_BASELINE_LAYERED_FILE` + `PARITY_BASELINE_RECURSIVE_SCENARIOS_FILE`; scenario files auto-filter current-side rows to each scenario's latest run config tuple to avoid mixed-window contamination)
- `update_parity_baseline.sh [source] [target]` (default `tests/results_layered_issue_scenarios.csv` -> `tests/baselines/layered_issue_scenarios.csv`)
- `update_parity_recursive_scenarios_baseline.sh [source] [target]` (default `tests/results_recursive_layout_scenarios.csv` -> `tests/baselines/recursive_layout_scenarios.csv`)
- Baseline lifecycle rules are documented in `tests/baselines/POLICY.md`.
- `run_parity_and_compare.sh [window] [mode]` (parity + compare + summary)
- `run_parity_and_check.sh [threshold] [window] [mode]` (parity + compare + summary + regression gate)
- `compare_java_parity_results.sh [rust_file] [java_file] [window] [output]` (generates a Java-vs-Rust comparison report for layered issue scenarios)
- `check_java_parity.sh [rust_file] [java_file] [window] [threshold]` (Java parity regression gate; default threshold is 0%)
- `check_java_parity_scenarios.sh [rust_file] [java_file] [window] [thresholds_file]` (per-scenario Java parity gate; default threshold file is `tests/java_parity_thresholds.csv`)
- `suggest_java_parity_thresholds.sh [rust_file] [java_file] [window] [buffer_pct] [output]` (generates per-scenario threshold candidate CSV from recent Rust-vs-Java window regressions; default output is `tests/java_parity_thresholds.suggested.csv`)
- `apply_java_parity_thresholds.sh [source] [target]` (validates and applies a threshold candidate CSV to the operational threshold CSV; default `tests/java_parity_thresholds.suggested.csv` -> `tests/java_parity_thresholds.csv`)
- `check_java_parity_artifacts.sh [java_file] [report_file]` (validates Java compare CSV/report artifacts plus minimum data-row and scenario-coverage gates; optional headers are skipped, and the effective minimum row count is `max(JAVA_ARTIFACT_MIN_ROWS, required_scenario_count)`)
- `update_java_parity_baseline.sh [source] [target]` (default `tests/java_results_layered_issue_scenarios.csv` -> `tests/baselines/java_layered_issue_scenarios.csv`)
- `summarize_java_parity_status.sh [results_report] [baseline_report] [java_results_file] [java_baseline_file] [output]` (generates a Java compare status/next-action summary report; default `tests/java_parity_status.md`)
- `export_java_baseline_candidate.sh [source] [target] [report]` (copies Java result CSV to a baseline-candidate path and writes a status report; if policy validation fails, behavior follows `JAVA_CANDIDATE_STRICT`)
- `check_java_baseline_candidate.sh [candidate] [rust_file] [window] [threshold] [report]` (checks candidate promotion readiness: artifact policy + Rust compare/parity validation, default report `tests/java_baseline_candidate_check.md`)
- `run_parity_and_compare_java.sh [java_file] [window] [threshold] [output]` (runs Rust layered issue parity + optional Java CSV generation + Java compare/gates; supports `JAVA_PARITY_COMPARE_MODE=results|baseline|both`)
- `check_core_options_parity.sh [report]` (compares Java `CoreOptions.java` with Rust `core_options.rs`/`core_options_meta.rs` to detect option/category drift and non-qualified `set_category_id`; default report `tests/core_options_parity.md`)
- `check_core_option_dependency_parity.sh [report]` (compares Java `addDependency` and Rust `add_dependency` in `CoreOptions` metadata by source-target-value; default report `tests/core_option_dependency_parity.md`; with `CORE_OPTION_DEPENDENCY_PARITY_STRICT=true`, drift fails the run)
- `check_algorithm_id_parity.sh [report]` (compares Java/Rust `ALGORITHM_ID` sets to report missing/extra IDs; default `tests/algorithm_id_parity.md`; strict mode via `ALGORITHM_ID_PARITY_STRICT=true`)
- `check_algorithm_category_parity.sh [report]` (compares Java algorithm categories from `*.Options.java` and Rust `set_category_id`; default `tests/algorithm_category_parity.md`; strict mode via `ALGORITHM_CATEGORY_PARITY_STRICT=true`)
- `check_algorithm_name_parity.sh [report]` (compares Java/Rust algorithm metadata `name`; default `tests/algorithm_name_parity.md`; strict mode via `ALGORITHM_NAME_PARITY_STRICT=true`)
- `check_algorithm_description_parity.sh [report]` (compares Java/Rust algorithm metadata `description`; default `tests/algorithm_description_parity.md`; strict mode via `ALGORITHM_DESCRIPTION_PARITY_STRICT=true`)
- `check_algorithm_option_support_parity.sh [report]` (compares Java `addOptionSupport` counts with Rust `add_option_support` + core `add_known_option_default`; for provider-registered algorithms, duplicated core bootstrap counts are excluded; default `tests/algorithm_option_support_parity.md`; strict mode via `ALGORITHM_OPTION_SUPPORT_PARITY_STRICT=true`)
- `check_algorithm_option_default_parity.sh [report]` (compares option-default semantics per option id: Java modes `explicit_null`/`explicit_nonnull`/`getDefault_*` vs Rust modes `none`/`nonnull`; `getDefault` accepts either Rust mode; explicit mismatch/unknown drive drift; missing/extra option pairs are informational; duplicated core bootstrap counts are excluded for provider-registered algorithms; default `tests/algorithm_option_default_parity.md`; strict mode via `ALGORITHM_OPTION_DEFAULT_PARITY_STRICT=true`; keep intermediate TSVs with `ALGORITHM_OPTION_DEFAULT_PARITY_KEEP_TMP=true`)
- `check_algorithm_option_default_value_parity.sh [report]` (compares Java/Rust option-definition defaults at normalized type/constant level (`null`/`bool`/`number`/`string`/`enum`) for option IDs used by Java `addOptionSupport`; only comparable mismatches drive drift, uncomparable forms are informational; default `tests/algorithm_option_default_value_parity.md`; strict mode via `ALGORITHM_OPTION_DEFAULT_VALUE_PARITY_STRICT=true`; keep intermediate TSVs with `ALGORITHM_OPTION_DEFAULT_VALUE_PARITY_KEEP_TMP=true`)
- `check_algorithm_feature_parity.sh [report]` (compares Java `supportedFeatures` and Rust `add_supported_feature` by algorithm-feature pairs; default `tests/algorithm_feature_parity.md`; strict mode via `ALGORITHM_FEATURE_PARITY_STRICT=true`)
- `check_algorithm_metadata_parity.sh [report]` (compares Java metadata fields in `*.Options.java` (`category`, `melkBundleName`, `definingBundleId`, `imagePath`) against Rust `LayoutAlgorithmData`; default `tests/algorithm_metadata_parity.md`; strict mode via `ALGORITHM_METADATA_PARITY_STRICT=true`)
- `check_layered_issue_test_parity.sh [report]` (compares Java layered issue test methods (`@Test`/`@TestAfterProcessor`) and Rust `#[test]` counts by issue file; default `tests/layered_issue_test_parity.md`; strict mode via `LAYERED_ISSUE_TEST_PARITY_STRICT=true`)
- `check_java_test_module_parity.sh [report]` (builds a Java↔Rust module-level test matrix from `external/elk/test` and `plugins/*`, reporting per-module test class/method counts and direct-map deltas; default `tests/java_test_module_parity.md`)
- `check_layered_phase_wiring_parity.sh [report]` (compares Java `GraphConfigurator` and Rust `graph_configurator` phase wiring rows (`before`/`after`, phase, processor, guard signature), emits detailed TSV artifacts under `tests/layered_phase_wiring/`; default report `tests/layered_phase_wiring_parity.md`; strict mode via `LAYERED_PHASE_WIRING_PARITY_STRICT=true`)
- `clean_parity_temp.sh [--apply] [--include-tracked] [--root <perf_dir>]` (cleans runtime TEMP artifacts under `tests/`; default is dry-run and skips tracked files, `--include-tracked` enables legacy tracked payload cleanup)
- `update_ptolemy_coverage_agents.sh` (runs `node_promotion_test`의 external ptolemy parse coverage/model-order validated count를 수집해 `AGENTS.md` 진행 기록에 배치별 정량 항목을 자동 추가)
- `run_perf_benchmark.sh [mode] [iterations] [warmup] [output_dir]` (5-way performance benchmark orchestration: Rust native, Rust API, Java, elkjs, NAPI, WASM; `mode` is `synthetic` (default) or `models`; outputs per-engine CSV + comparison report)
- `run_java_model_benchmark.sh [mode] [iterations] [warmup] [output]` (Java model benchmark via `ElkModelBenchTest`; supports `synthetic` and `models` modes; follows same isolation/injection pattern as `run_java_parity_layered_issue_scenarios.sh`)
- `compare_perf_results.py [results_dir] [output]` (5-way CSV comparison report generator; reads per-engine CSV files, outputs markdown with summary + per-scenario tables; supports both new format with `engine` column and legacy format)
- `run_all_checks.sh [threshold] [window]` (cargo test, clippy, parity gate)
- `run_fast_checks.sh` (cargo test, clippy only)

`run_parity_all.sh` env overrides (defaults shown):

```
COMMENT_COUNT=2000
COMMENT_ITERATIONS=5
COMMENT_WARMUP=1
COMMENT_OUTPUT=tests/results_comment_attachment.csv
GRAPH_NODES=1000
GRAPH_EDGES=2000
GRAPH_ITERATIONS=5
GRAPH_WARMUP=1
GRAPH_MODE=both
GRAPH_OUTPUT=tests/results_graph_validation.csv
LAYOUT_NODES=500
LAYOUT_EDGES=1000
LAYOUT_ITERATIONS=5
LAYOUT_WARMUP=1
LAYOUT_ALGORITHM=fixed
LAYOUT_VALIDATE_GRAPH=false
LAYOUT_VALIDATE_OPTIONS=false
LAYOUT_OUTPUT=tests/results_recursive_layout.csv
LAYOUT_LAYERED_OUTPUT=tests/results_recursive_layout_layered.csv
RECURSIVE_SCENARIO_PROFILE=default
RECURSIVE_SCENARIOS=
RECURSIVE_SCENARIO_ITERATIONS=5
RECURSIVE_SCENARIO_WARMUP=1
RECURSIVE_SCENARIO_OUTPUT=tests/results_recursive_layout_scenarios.csv
LAYERED_ISSUE_SCENARIOS=issue_405,issue_603,issue_680,issue_871,issue_905
LAYERED_ISSUE_ITERATIONS=20
LAYERED_ISSUE_WARMUP=3
LAYERED_ISSUE_OUTPUT=tests/results_layered_issue_scenarios.csv
```

CI workflows (GitHub Actions):
- `.github/workflows/ci.yml` runs `run_fast_checks.sh` on push/PR.
- `.github/workflows/parity.yml` runs parity scripts on manual dispatch and uploads CSV/summary artifacts.
- In `.github/workflows/parity.yml`, enabling `recursive_runtime_budget_gate=true` runs `check_recursive_parity_runtime_budget.sh`, generates `tests/recursive_runtime_budget.md`, and blocks the gate on profile budget overruns.
- Default Java path in `.github/workflows/parity.yml` is set to strict values (`java_compare_enabled=true`, `java_compare_mode=both`, `java_generate_enabled=true`, `java_export_baseline_candidate=true`, `java_export_candidate_strict=true`, `java_parity_gate=true`, `java_baseline_parity_gate=true`).
- `.github/workflows/parity.yml` runs Java steps with `JAVA_PARITY_EXTERNAL_ISOLATE=true` to isolate execution from the original `external/elk` tree.
- `.github/workflows/parity.yml` validates Java compare artifacts with `check_java_parity_artifacts.sh` when Java compare is enabled.
- `.github/workflows/parity.yml` generates parity reports `tests/core_options_parity.md`, `tests/core_option_dependency_parity.md`, `tests/algorithm_id_parity.md`, `tests/algorithm_category_parity.md`, `tests/algorithm_name_parity.md`, `tests/algorithm_description_parity.md`, `tests/algorithm_option_support_parity.md`, `tests/algorithm_option_default_parity.md`, `tests/algorithm_option_default_value_parity.md`, `tests/algorithm_feature_parity.md`, `tests/algorithm_metadata_parity.md`, `tests/layered_phase_wiring_parity.md` as artifacts.
- `.github/workflows/parity.yml` can include `tests/layered_issue_test_parity.md` when `check_layered_issue_test_parity.sh` is wired as a parity step.
- In `.github/workflows/parity.yml`, `java_generate_dry_run=true` skips Java compare/parity and only emits a dry-run summary report (`tests/java_vs_rust.md`).
- In `.github/workflows/parity.yml`, `java_compare_mode=baseline|both` adds baseline report generation (`tests/java_vs_rust_baseline.md`) and baseline parity gates.
- `.github/workflows/parity.yml` collects non-layered recursive scenario parity via the `recursive_scenarios` input (`tests/results_recursive_layout_scenarios.csv`).
- `.github/workflows/parity.yml` injects per-scenario Java parity threshold CSV through `java_parity_thresholds_file`.
- `.github/workflows/parity.yml` tunes Java CSV minimum-row gates via `java_artifact_min_rows`, and checks scenario coverage against `layered_issue_scenarios` (effective minimum is `max(java_artifact_min_rows, scenario_count)`).
- `.github/workflows/parity.yml` can control Java DNS preflight policy per runner environment via `java_skip_dns_check` / `java_required_hosts`.
- Java steps in `.github/workflows/parity.yml` are unified through single-wrapper invocation (`run_parity_and_compare_java.sh`) to keep local and CI behavior aligned.
- After Java pipeline execution, `.github/workflows/parity.yml` generates `tests/java_parity_status.md` and uploads it as an artifact with result/skip status and baseline-update next actions (including candidate file/report state).
- With `java_export_baseline_candidate=true`, `.github/workflows/parity.yml` also uploads `tests/java_baseline_candidate_status.md` and `java_baseline_candidate_file` CSV as artifacts.
- With `java_export_baseline_candidate=true`, `.github/workflows/parity.yml` runs `check_java_baseline_candidate.sh`, generates `tests/java_baseline_candidate_check.md`, and uploads it as an artifact (`java_export_candidate_strict` controls strict failure behavior).
- For Java failure triage, see `tests/java_parity_triage.md`.

Java parity generation env overrides:

```
JAVA_PARITY_GENERATE=false
JAVA_PARITY_SCENARIOS=$LAYERED_ISSUE_SCENARIOS
JAVA_PARITY_ITERATIONS=$LAYERED_ISSUE_ITERATIONS
JAVA_PARITY_WARMUP=$LAYERED_ISSUE_WARMUP
JAVA_PARITY_OUTPUT=$JAVA_FILE
JAVA_PARITY_RESET_OUTPUT=true
JAVA_PARITY_DRY_RUN=false
JAVA_PARITY_VERIFY_ARTIFACTS=true
JAVA_PARITY_ALLOW_GENERATE_FAILURE=false
JAVA_PARITY_RETRIES=0
JAVA_PARITY_RETRY_DELAY_SECS=3
JAVA_PARITY_COMPARE_MODE=results
JAVA_BASELINE_FILE=tests/baselines/java_layered_issue_scenarios.csv
JAVA_BASELINE_OUTPUT=tests/java_vs_rust_baseline.md
JAVA_BASELINE_THRESHOLD=$THRESHOLD
JAVA_PARITY_SCENARIO_THRESHOLDS_FILE=tests/java_parity_thresholds.csv
JAVA_RESULTS_PARITY_GATE=true
JAVA_BASELINE_PARITY_GATE=true
JAVA_ARTIFACT_MIN_ROWS=1
JAVA_ARTIFACT_REQUIRED_SCENARIOS=$LAYERED_ISSUE_SCENARIOS
JAVA_PARITY_MVN_BIN=mvn
JAVA_PARITY_BUILD_PLUGINS=true
JAVA_PARITY_EXTERNAL_ELK_ROOT=external/elk
JAVA_PARITY_EXTERNAL_ISOLATE=true
JAVA_PARITY_EXTERNAL_WORKTREE_ROOT=/tmp
JAVA_PARITY_PREPARE_POM=<auto:$JAVA_PARITY_EXTERNAL_ELK_ROOT/build/pom.xml or isolated worktree>
JAVA_PARITY_PREPARE_MODULES=
JAVA_PARITY_TEST_POM=<auto:$JAVA_PARITY_EXTERNAL_ELK_ROOT/build/pom.xml or isolated worktree>
JAVA_PARITY_TEST_MODULES=../test/org.eclipse.elk.alg.test,../test/org.eclipse.elk.alg.layered.test
JAVA_PARITY_TEST_CLASS=LayeredIssueParityBenchTest
JAVA_PARITY_TEST_METHOD=
JAVA_PARITY_TEST_GOAL=verify
JAVA_PARITY_BENCH_INJECT=true
JAVA_PARITY_BENCH_SOURCE=scripts/java/LayeredIssueParityBenchTest.java
JAVA_PARITY_BENCH_DEST=<auto:$JAVA_PARITY_EXTERNAL_ELK_ROOT/test/... or isolated worktree>
JAVA_PARITY_BENCH_CLEANUP=true
JAVA_PARITY_PREPARE_ARGS="-DskipTests -DskipITs"
JAVA_PARITY_MVN_LOCAL_REPO=
JAVA_PARITY_MVN_ARGS=
JAVA_PARITY_SKIP_DNS_CHECK=false
JAVA_PARITY_REQUIRED_HOSTS=repo.eclipse.org,repo.maven.apache.org
```

Model parity env overrides:

```
JAVA_PARITY_DRY_RUN=false
JAVA_PARITY_EXTERNAL_ISOLATE=true
JAVA_PARITY_REQUIRE_CLEAN_EXTERNAL_ELK=true
JAVA_PARITY_BUILD_PLUGINS=true
JAVA_PARITY_MVN_LOCAL_REPO=
JAVA_PARITY_LIMIT=0
JAVA_PARITY_INCLUDE=
JAVA_PARITY_FAIL_FAST=false
JAVA_PARITY_PRETTY_PRINT=false
JAVA_PARITY_RESET_OUTPUT=true
MODEL_PARITY_PRETTY_PRINT=false
MODEL_PARITY_STOP_ON_ERROR=false
MODEL_PARITY_ABS_TOL=1e-6
MODEL_PARITY_MAX_DIFFS_PER_MODEL=20
MODEL_PARITY_STRICT=false
MODEL_PARITY_SKIP_JAVA_EXPORT=false
```

Notes:
- Even if `JAVA_ARTIFACT_MIN_ROWS` is configured low, the effective minimum does not go below the scenario count in `JAVA_ARTIFACT_REQUIRED_SCENARIOS`.
- In `run_parity_and_compare_java.sh`, when `JAVA_PARITY_GENERATE=true` and `JAVA_PARITY_MVN_LOCAL_REPO` is empty, a per-run temporary path (`${TMPDIR:-/tmp}/m2-java-parity-${USER:-user}-$$`) is auto-selected to avoid lock contention.
- With `JAVA_PARITY_COMPARE_MODE=baseline`, compare/gates run only against `JAVA_BASELINE_FILE`; Java CSV generation is optional.
- In `JAVA_PARITY_COMPARE_MODE=baseline`, Java generation is automatically skipped even when `JAVA_PARITY_GENERATE=true`.
- If `JAVA_PARITY_SCENARIO_THRESHOLDS_FILE` exists, Java parity uses per-scenario thresholds (for example `tests/java_parity_thresholds.csv`); if missing, it falls back to single global `THRESHOLD`.
- With `LAYERED_ISSUE_SKIP_RUST_RUN=true`, Rust layered parity rerun is skipped and Java compare runs against the existing `LAYERED_ISSUE_OUTPUT` file (used in CI integration stages).
- With `JAVA_PARITY_ALLOW_GENERATE_FAILURE=true`, generation failure is converted into a skip report for results compare (in `both` mode, baseline compare still runs), while preserving wrapper exit behavior.
- `JAVA_PARITY_RETRIES` / `JAVA_PARITY_RETRY_DELAY_SECS` tune retry policy for Java Maven commands.
- `run_java_parity_layered_issue_scenarios.sh` performs DNS preflight by default and fails early when `repo.eclipse.org` or `repo.maven.apache.org` cannot be resolved (`JAVA_PARITY_SKIP_DNS_CHECK=true` bypasses this check).
- `run_java_parity_layered_issue_scenarios.sh` runs in an isolated temporary directory by default (`JAVA_PARITY_EXTERNAL_ISOLATE=true`; git worktree first, temporary copy fallback).
- `run_java_model_parity_export.sh` also defaults to isolated execution (`JAVA_PARITY_EXTERNAL_ISOLATE=true`) and restores/removes the injected Java class automatically.
- `run_java_model_parity_export.sh` refuses to run when `external/elk` is dirty by default (`JAVA_PARITY_REQUIRE_CLEAN_EXTERNAL_ELK=true`); set it to `false` only when you intentionally want to include local Java changes.
- `MODEL_PARITY_SKIP_JAVA_EXPORT=true` skips Java export and reuses the existing Java manifest/layout baseline (fails fast when `tests/model_parity/java/java_manifest.tsv` is missing).
- `run_model_parity_elk_vs_rust.sh` reads Java manifest `tests/model_parity/java/java_manifest.tsv`, writes Rust manifest `tests/model_parity/rust_manifest.tsv`, and emits `tests/model_parity/report.md`.
- Under defaults, the original `external/elk` worktree remains unchanged after runs (set `JAVA_PARITY_EXTERNAL_ISOLATE=false` for direct-in-place execution).
- Model parity strict gate can be enabled with `MODEL_PARITY_STRICT=true` (non-zero exit when drift/errors exist).
- Baseline candidate export/readiness checks can be tuned with `JAVA_CANDIDATE_MIN_ROWS`, `JAVA_CANDIDATE_REQUIRED_SCENARIOS`, `JAVA_CANDIDATE_REQUIRE_PARITY`, and `JAVA_CANDIDATE_STRICT`.

Repeated-run tips:
- On the first run, prepare local Maven/Tycho artifacts with `JAVA_PARITY_BUILD_PLUGINS=true`.
- For repeated runs, use `JAVA_PARITY_BUILD_PLUGINS=false` to run only the test phase and refresh Java CSV much faster.
- In CI, set `JAVA_PARITY_MVN_LOCAL_REPO` to a run-scoped temporary directory to avoid Tycho lock contention
  (example: `${RUNNER_TEMP}/m2-java-parity-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}`).

Java parity CI operating guide (finalized):
- Any workflow with `java_generate_enabled=true` should isolate `JAVA_PARITY_MVN_LOCAL_REPO` per run-attempt.
- Within one job, keep prepare/install and test phases on the same `JAVA_PARITY_MVN_LOCAL_REPO` to maximize artifact reuse.
- Do not share the same path across concurrent jobs/runners (no shared `.m2` path).
- In the same run context, when only Java CSV refresh is needed, switch to `JAVA_PARITY_BUILD_PLUGINS=false` to reduce lock wait time.
