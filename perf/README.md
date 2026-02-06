Performance results are appended by scripts in `scripts/`.

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

Compare helper:
- `scripts/compare_perf_results.sh [window]` prints a quick diff between the last two windows (default window 1).
- `PERF_COMPARE_MODE=baseline` (or `both`) enables scenario-level comparison against `PERF_BASELINE_LAYERED_FILE` (default `perf/baselines/layered_issue_scenarios.csv`).
- `scripts/summarize_perf_results.sh` writes `perf/summary.md` with the latest run and the last 5 runs for each perf test.
- `scripts/check_perf_regression.sh` exits non-zero when avg_ms or ops/elems per sec regress more than a threshold (default 5%), using windowed averages (default window 3; needs 2*window lines).
- `scripts/update_perf_baseline.sh [source] [target]` updates the layered issue baseline file used by baseline compare/check flows.
- Baseline lifecycle and CI usage rules are documented in `perf/baselines/POLICY.md`.
- `scripts/run_perf_and_check.sh [threshold] [window] [mode]` runs all perf scripts, compares, summarizes, then checks regressions.
- `scripts/run_perf_and_compare.sh [window] [mode]` runs all perf scripts, compares with the given window/mode, then summarizes.
- `scripts/run_perf_layered_layout.sh` runs recursive layout perf with the layered algorithm (default output `perf/results_recursive_layout_layered.csv`).
- `scripts/compare_java_perf_results.sh` and `scripts/check_java_perf_parity.sh` compare Rust layered issue results against Java CSVs (default Java input: `perf/java_results_layered_issue_scenarios.csv`).
- `scripts/check_java_perf_artifacts.sh` validates Java compare artifacts (report exists, Java CSV exists when required, minimum data rows, and required scenario coverage; optional header rows are ignored, minimum rows use `max(JAVA_ARTIFACT_MIN_ROWS, required_scenario_count)`).
- `scripts/update_java_perf_baseline.sh [source] [target]` updates the Java layered issue baseline CSV (default target `perf/baselines/java_layered_issue_scenarios.csv`).
- `scripts/summarize_java_perf_status.sh` writes a Java pipeline status report (default `perf/java_perf_status.md`) with baseline update guidance.
- `scripts/export_java_baseline_candidate.sh` exports Java results to a baseline-candidate path (default `perf/baselines/java_layered_issue_scenarios.candidate.csv`) and writes a status report (`perf/java_baseline_candidate_status.md` by default).
- `scripts/check_java_baseline_candidate.sh` validates candidate promotion readiness (artifact policy + Rust compare/parity) and writes `perf/java_baseline_candidate_check.md`.
- `scripts/run_java_perf_layered_issue_scenarios.sh` generates the Java layered issue CSV via external ELK Tycho test (`LayeredIssuePerfBenchTest`); by default it runs in an isolated copy of `external/elk` (git worktree when available, copy fallback otherwise) and cleans up after completion so the original `external/elk` tree remains unchanged.
- Perf workflow Java 운영 기본값/실패 대응 절차는 `perf/JAVA_PERF_TRIAGE.md`를 기준으로 관리한다.
- `scripts/run_perf_and_compare_java.sh` can generate Java CSV before compare/check when `JAVA_PERF_GENERATE=true` (use `JAVA_PERF_DRY_RUN=true` to print the Maven command only) and supports `JAVA_PERF_COMPARE_MODE=results|baseline|both`.
- When `JAVA_PERF_GENERATE=true` and `JAVA_PERF_DRY_RUN=true`, `run_perf_and_compare_java.sh` writes a dry-run summary report and skips compare/parity if no Java CSV exists.
- `JAVA_PERF_ALLOW_GENERATE_FAILURE=true` lets the wrapper continue after generation failure (results compare becomes a skip report; baseline compare can still run in `both` mode).
- `JAVA_PERF_RETRIES` and `JAVA_PERF_RETRY_DELAY_SECS` control retry behavior for Java Maven commands during generation.
- Java generation runs DNS preflight for `repo.eclipse.org` and `repo.maven.apache.org` by default and fails fast when hosts are unreachable (`JAVA_PERF_SKIP_DNS_CHECK=true` bypasses preflight).
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
