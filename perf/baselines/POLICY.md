# Layered Issue Baseline Policy

This document defines how `perf/baselines/layered_issue_scenarios.csv` is produced and maintained.

## Scope

- Baseline target: `perf/results_layered_issue_scenarios.csv` schema.
- Scenario key: `scenario` column (second column).
- Metrics used by compare/check scripts:
  - `avg_ms` (column 6)
  - `scenarios_per_sec` (column 7)

## Update Rules

1. Use a stable branch tip (typically `main`) with no pending performance-related local changes.
2. Run at least 5 iterations with warmup (recommended: `--iterations 20 --warmup 3`).
3. Execute at least 3 repeated runs and inspect variance before accepting a new baseline.
4. Update baseline only after intentional performance work or infrastructure changes that materially shift timing.
5. Keep scenario coverage aligned with `LAYERED_ISSUE_SCENARIOS` defaults unless explicitly changed.

## Update Procedure

1. Generate fresh results:
   - `sh scripts/run_perf_layered_issue_scenarios.sh "issue_405,issue_603,issue_680,issue_871,issue_905" 20 3 perf/results_layered_issue_scenarios.csv`
2. Copy to baseline:
   - `sh scripts/update_perf_baseline.sh`
3. Verify:
   - `PERF_COMPARE_MODE=baseline sh scripts/compare_perf_results.sh 3`
   - `PERF_COMPARE_MODE=baseline sh scripts/check_perf_regression.sh 5 3`
4. Commit `perf/baselines/layered_issue_scenarios.csv` with a short note describing why the baseline changed.

## CI Usage

- Use `perf_compare_mode=window` when baseline is intentionally not provided.
- Use `perf_compare_mode=baseline` or `both` only when `baseline_layered_file` points to a valid CSV.
- Enable `regression_gate=true` only after baseline quality is confirmed.

## Branch / Gate Defaults

- Default PR runs:
  - `perf_compare_mode=window`
  - `regression_gate=false`
  - `regression_threshold=5`
  - `regression_window=3`
- Baseline validation runs (maintainer-triggered, typically on `main`):
  - `perf_compare_mode=both`
  - `regression_gate=true`
  - `regression_threshold=5`
  - `regression_window=3`
- Java parity:
  - `java_compare_enabled=true` when Java CSV exists or `java_generate_enabled=true`.
  - Keep `java_parity_gate=false` until CI stability is confirmed; then enable with a team-agreed threshold.

## Java CI Lock Avoidance

- When `java_generate_enabled=true`, isolate Maven/Tycho metadata with a per-run local repository.
- Recommended env in CI:
  - `JAVA_PERF_MVN_LOCAL_REPO=${RUNNER_TEMP}/m2-java-perf-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}`
- This avoids `.m2` metadata lock conflicts (`p2-artifacts.properties.tycholock`) across concurrent jobs/runs.

## Java Baseline (Optional)

- If Java parity is tracked against a pinned Java CSV, keep it at `perf/baselines/java_layered_issue_scenarios.csv`.
- Update flow:
  1. Generate Java CSV (`java_generate_enabled=true` in CI or local Maven run with matching scenarios).
  2. Export candidate (`sh scripts/export_java_baseline_candidate.sh`) and review readiness (`sh scripts/check_java_baseline_candidate.sh`).
  3. Promote candidate (`sh scripts/update_java_perf_baseline.sh`).
  4. Re-run Java compare/parity checks against the updated baseline CSV and review drift.
- CI toggle:
  - `java_compare_enabled=true`
  - `java_compare_mode=baseline` (or `both`)
  - `java_baseline_file=perf/baselines/java_layered_issue_scenarios.csv`
  - optionally `java_allow_generate_failure=true` to let `both` mode proceed with baseline checks when fresh Java generation is temporarily unavailable
  - optionally tune `java_generate_retries` / `java_generate_retry_delay_secs` for transient Maven/Tycho fetch failures
  - optionally `java_export_baseline_candidate=true` to publish baseline-candidate artifact (`java_baseline_candidate_file`) for manual promotion
  - optionally `java_export_candidate_strict=true` to fail candidate export/readiness check when policy validation fails
