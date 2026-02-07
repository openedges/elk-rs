Performance results are appended by scripts in `scripts/`.

Release quick guide:
- Run the pre-release checklist in `RELEASE_CHECKLIST.md`.
- A failure in `PERF_COMPARE_MODE=baseline sh scripts/check_perf_regression.sh 5 3` means a "Rust current vs Rust baseline" regression.
- That failure does not directly mean Rust is slower than Java. Java comparison is evaluated separately via `check_java_perf_parity*.sh`.
- Release decisions should consider code quality (`cargo test/clippy/build`), parity reports (`status: ok`), and performance gates together.

`results_comment_attachment.csv` columns:
- unix_timestamp_seconds
- count
- iterations
- warmup
- elapsed_nanos_total
- avg_iteration_ms
- ops_per_sec

`results_graph_validation.csv` columns:
- unix_timestamp_seconds
- mode
- nodes
- edges
- iterations
- warmup
- elapsed_nanos_total
- avg_iteration_ms
- elems_per_sec

`results_recursive_layout.csv` columns:
- unix_timestamp_seconds
- algorithm
- nodes
- edges
- iterations
- warmup
- elapsed_nanos_total
- avg_iteration_ms
- elems_per_sec
- validate_graph
- validate_options

`results_recursive_layout_layered.csv` columns:
- unix_timestamp_seconds
- algorithm
- nodes
- edges
- iterations
- warmup
- elapsed_nanos_total
- avg_iteration_ms
- elems_per_sec
- validate_graph
- validate_options

`results_recursive_layout_scenarios.csv` columns:
- unix_timestamp_seconds
- scenario
- algorithm
- nodes
- edges
- iterations
- warmup
- elapsed_nanos_total
- avg_iteration_ms
- elems_per_sec
- validate_graph
- validate_options

Compare helper:
- `scripts/run_perf_recursive_layout_scenarios.sh` appends non-layered recursive layout scenario runs to `perf/results_recursive_layout_scenarios.csv` (`PERF_RECURSIVE_SCENARIO_PROFILE=default` uses `fixed_dense,fixed_sparse,random_dense,random_sparse,box_sparse,fixed_validated,random_validated,box_validated`; `quick` uses a reduced set; `full` includes `box_large`; explicit scenario arguments take precedence over profile defaults).
- `scripts/compare_perf_results.sh [window]` prints a quick diff between the last two windows (default window 1).
- For scenario-based files (`recursive_layout_scenarios`, `layered_issue_scenarios`), compare/regression scripts auto-filter current-side rows to each scenario's latest run config tuple to avoid mixed-window contamination from different `iterations/warmup` runs.
- `scripts/check_recursive_perf_runtime_budget.sh [results_file] [profile] [report]` checks latest recursive scenario `avg_iteration_ms` values against profile budgets (`quick|default|full`; defaults `40/60/120ms`) and writes `perf/recursive_runtime_budget.md`.
- `PERF_COMPARE_MODE=baseline` (or `both`) enables scenario-level comparison against `PERF_BASELINE_LAYERED_FILE` (default `perf/baselines/layered_issue_scenarios.csv`) and `PERF_BASELINE_RECURSIVE_SCENARIOS_FILE` (default `perf/baselines/recursive_layout_scenarios.csv`).
- `scripts/summarize_perf_results.sh` writes `perf/summary.md` with the latest run and the last 5 runs for each perf test.
- `scripts/check_perf_regression.sh` exits non-zero when avg_ms or ops/elems per sec regress more than a threshold (default 5%), using windowed averages (default window 3; needs 2*window lines).
- `scripts/update_perf_baseline.sh [source] [target]` updates the layered issue baseline file used by baseline compare/check flows.
- `scripts/update_perf_recursive_scenarios_baseline.sh [source] [target]` updates the recursive scenario baseline file (default `perf/results_recursive_layout_scenarios.csv` -> `perf/baselines/recursive_layout_scenarios.csv`).
- Baseline lifecycle and CI usage rules are documented in `perf/baselines/POLICY.md`.
- `scripts/run_perf_and_check.sh [threshold] [window] [mode]` runs all perf scripts, compares, summarizes, then checks regressions.
- `scripts/run_perf_and_compare.sh [window] [mode]` runs all perf scripts, compares with the given window/mode, then summarizes.
- `scripts/run_perf_layered_layout.sh` runs recursive layout perf with the layered algorithm (default output `perf/results_recursive_layout_layered.csv`).
- `scripts/compare_java_perf_results.sh` and `scripts/check_java_perf_parity.sh` compare Rust layered issue results against Java CSVs (default Java input: `perf/java_results_layered_issue_scenarios.csv`).
- `scripts/check_java_perf_artifacts.sh` validates Java compare artifacts (report exists, Java CSV exists when required, minimum data rows, and required scenario coverage; optional header rows are ignored, minimum rows use `max(JAVA_ARTIFACT_MIN_ROWS, required_scenario_count)`).
- `scripts/check_java_perf_parity_scenarios.sh` applies per-scenario Java parity gates based on threshold CSV rows (`scenario,max_avg_ms_regression_pct,max_scenarios_per_sec_regression_pct`; default file `perf/java_parity_thresholds.csv`).
- `scripts/suggest_java_parity_thresholds.sh` generates a threshold-candidate CSV (`perf/java_parity_thresholds.suggested.csv` by default) from recent Rust/Java window averages; use it to calibrate `perf/java_parity_thresholds.csv`.
- `scripts/apply_java_parity_thresholds.sh` validates and applies a threshold-candidate CSV to the operational threshold file (`perf/java_parity_thresholds.suggested.csv` -> `perf/java_parity_thresholds.csv` by default).
- `scripts/run_perf_and_compare_java.sh` uses `JAVA_PARITY_SCENARIO_THRESHOLDS_FILE` when present and falls back to global `THRESHOLD` gating when the file is missing.
- `scripts/update_java_perf_baseline.sh [source] [target]` updates the Java layered issue baseline CSV (default target `perf/baselines/java_layered_issue_scenarios.csv`).
- `scripts/summarize_java_perf_status.sh` writes a Java pipeline status report (default `perf/java_perf_status.md`) with baseline update guidance.
- `scripts/export_java_baseline_candidate.sh` exports Java results to a baseline-candidate path (default `perf/baselines/java_layered_issue_scenarios.candidate.csv`) and writes a status report (`perf/java_baseline_candidate_status.md` by default).
- `scripts/check_java_baseline_candidate.sh` validates candidate promotion readiness (artifact policy + Rust compare/parity) and writes `perf/java_baseline_candidate_check.md`.
- `scripts/check_algorithm_id_parity.sh` compares Java/Rust `ALGORITHM_ID` sets and writes `perf/algorithm_id_parity.md` (set `ALGORITHM_ID_PARITY_STRICT=true` to fail on drift).
- `scripts/check_algorithm_category_parity.sh` compares Java/Rust algorithm category IDs and writes `perf/algorithm_category_parity.md` (set `ALGORITHM_CATEGORY_PARITY_STRICT=true` to fail on drift).
- `scripts/check_algorithm_name_parity.sh` compares Java/Rust algorithm metadata `name` strings and writes `perf/algorithm_name_parity.md` (set `ALGORITHM_NAME_PARITY_STRICT=true` to fail on drift).
- `scripts/check_algorithm_description_parity.sh` compares Java/Rust algorithm metadata `description` strings and writes `perf/algorithm_description_parity.md` (set `ALGORITHM_DESCRIPTION_PARITY_STRICT=true` to fail on drift).
- `scripts/check_algorithm_option_support_parity.sh` compares Java/Rust algorithm option-support registration counts (`addOptionSupport` vs `add_option_support` + core `add_known_option_default`) and writes `perf/algorithm_option_support_parity.md` (provider-registered algorithms automatically exclude duplicated core bootstrap counts; set `ALGORITHM_OPTION_SUPPORT_PARITY_STRICT=true` to fail on drift).
- `scripts/check_algorithm_option_default_parity.sh` compares Java/Rust algorithm option-default semantics per option id (`addOptionSupport` mode `explicit_null`/`explicit_nonnull`/`getDefault_*` vs Rust `add_option_support` + core `add_known_option_default` mode `none`/`nonnull`) and writes `perf/algorithm_option_default_parity.md` (`getDefault` entries allow either Rust mode, explicit mismatch/unknown drive drift, and missing/extra option-pairs are reported as informational; provider-registered algorithms automatically exclude duplicated core bootstrap counts; set `ALGORITHM_OPTION_DEFAULT_PARITY_STRICT=true` to fail on drift; set `ALGORITHM_OPTION_DEFAULT_PARITY_KEEP_TMP=true` to keep intermediate TSV files for parser/debug inspection).
- `scripts/check_algorithm_option_default_value_parity.sh` compares Java/Rust option-definition defaults at normalized type/constant level (`null`/`bool`/`number`/`string`/`enum`) for option IDs used by Java `addOptionSupport`, writes `perf/algorithm_option_default_value_parity.md`, treats only comparable mismatches as drift (uncomparable constructor/object forms are informational), and supports `ALGORITHM_OPTION_DEFAULT_VALUE_PARITY_STRICT=true` / `ALGORITHM_OPTION_DEFAULT_VALUE_PARITY_KEEP_TMP=true`.
- `scripts/check_core_option_dependency_parity.sh` compares Java/Rust Core option dependency registrations (`addDependency`/`add_dependency`) and writes `perf/core_option_dependency_parity.md` (set `CORE_OPTION_DEPENDENCY_PARITY_STRICT=true` to fail on drift).
- `scripts/check_algorithm_feature_parity.sh` compares Java/Rust algorithm supported-feature pairs (`supportedFeatures` vs `add_supported_feature`) and writes `perf/algorithm_feature_parity.md` (set `ALGORITHM_FEATURE_PARITY_STRICT=true` to fail on drift).
- `scripts/check_algorithm_metadata_parity.sh` compares Java/Rust algorithm metadata fields (`category`, `melkBundleName`/`bundle_name`, `definingBundleId`, `imagePath`) and writes `perf/algorithm_metadata_parity.md` (set `ALGORITHM_METADATA_PARITY_STRICT=true` to fail on drift).
- `scripts/check_layered_issue_test_parity.sh` compares Java layered issue test methods (`@Test`/`@TestAfterProcessor`) and Rust `#[test]` counts per issue file, writes `perf/layered_issue_test_parity.md`, and supports `LAYERED_ISSUE_TEST_PARITY_STRICT=true`.
- `scripts/check_java_test_module_parity.sh` generates a Java↔Rust test parity matrix at module level (`external/elk/test` vs `plugins/*`), writes `perf/java_test_module_parity.md`, and reports direct-map deltas plus no-direct modules.
- `scripts/run_java_perf_layered_issue_scenarios.sh` generates the Java layered issue CSV via external ELK Tycho test (`LayeredIssuePerfBenchTest`); by default it runs in an isolated copy of `external/elk` (git worktree when available, copy fallback otherwise) and cleans up after completion so the original `external/elk` tree remains unchanged.
- Use `perf/JAVA_PERF_TRIAGE.md` as the source of truth for Java perf workflow default settings and failure triage procedures.
- `scripts/run_perf_and_compare_java.sh` can generate Java CSV before compare/check when `JAVA_PERF_GENERATE=true` (use `JAVA_PERF_DRY_RUN=true` to print the Maven command only) and supports `JAVA_PERF_COMPARE_MODE=results|baseline|both`.
- When `JAVA_PERF_GENERATE=true` and `JAVA_PERF_DRY_RUN=true`, `run_perf_and_compare_java.sh` writes a dry-run summary report and skips compare/parity if no Java CSV exists.
- `JAVA_PERF_ALLOW_GENERATE_FAILURE=true` lets the wrapper continue after generation failure (results compare becomes a skip report; baseline compare can still run in `both` mode).
- `JAVA_PERF_RETRIES` and `JAVA_PERF_RETRY_DELAY_SECS` control retry behavior for Java Maven commands during generation.
- Java generation runs DNS preflight for `repo.eclipse.org` and `repo.maven.apache.org` by default and fails fast when hosts are unreachable (`JAVA_PERF_SKIP_DNS_CHECK=true` bypasses preflight).
- Perf workflow input `java_skip_dns_check`/`java_required_hosts` can override the DNS preflight policy without changing scripts.
- Java CSV generation can be split into prepare + fast rerun phases (`JAVA_PERF_BUILD_PLUGINS=true` once, then `JAVA_PERF_BUILD_PLUGINS=false` for repeat runs).
- For CI Java generation, set a per-run local Maven repository (`JAVA_PERF_MVN_LOCAL_REPO`) to avoid Tycho lock collisions on shared `.m2` metadata.
- For local generation via `run_perf_and_compare_java.sh`, if `JAVA_PERF_MVN_LOCAL_REPO` is unset it auto-allocates a per-run temp path to reduce Tycho lock contention.
- Baseline compare mode (`JAVA_PERF_COMPARE_MODE=baseline|both`) writes `perf/java_vs_rust_baseline.md` by default and can gate with `JAVA_BASELINE_THRESHOLD`.
- In `JAVA_PERF_COMPARE_MODE=baseline`, Java generation is skipped even if `JAVA_PERF_GENERATE=true`.
- CI `perf.yml` now calls the same wrapper script (`run_perf_and_compare_java.sh`) to keep local and CI Java compare behavior aligned.
- Optional CI toggles `java_export_baseline_candidate=true` and `java_export_candidate_strict=true` control baseline-candidate artifact export and strict validation behavior.
- When `java_export_baseline_candidate=true`, CI also runs baseline-candidate readiness checks and uploads `perf/java_baseline_candidate_check.md`.
- Java compare scripts assume the same scenario schema as Rust layered issue CSV (`scenario` in col 2, `avg_ms` in col 6, `scenarios_per_sec` in col 7); override with `JAVA_*_COL` env vars if needed.
- Quality gate knobs: `JAVA_ARTIFACT_MIN_ROWS` and `JAVA_ARTIFACT_REQUIRED_SCENARIOS` (comma-separated) control Java CSV artifact acceptance; the effective row floor is `max(JAVA_ARTIFACT_MIN_ROWS, required_scenario_count)`.

Java generation policy (CI):
- Use `JAVA_PERF_MVN_LOCAL_REPO=${RUNNER_TEMP}/m2-java-perf-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}` (or an equivalent unique path per run-attempt).
- Keep prepare/install and test phases on the same local repo path inside one job to reuse generated Tycho artifacts.
- Never share one local repo path across concurrent jobs.
- For reruns that only refresh Java CSV in the same run context, prefer `JAVA_PERF_BUILD_PLUGINS=false`.
