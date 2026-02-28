# Java Perf CI Triage

This guide is for failures in `.github/workflows/parity.yml` Java steps.

## Default CI Policy

- `java_compare_enabled=true`
- `java_compare_mode=both`
- `java_generate_enabled=true`
- `java_export_baseline_candidate=true`
- `java_export_candidate_strict=true`
- `java_parity_gate=true`
- `java_baseline_parity_gate=true`
- Java execution runs with external ELK isolation (`JAVA_PARITY_EXTERNAL_ISOLATE=true`).

The default path is strict by design: any generation/tests/candidate-readiness failure should fail the workflow.

## Failure Classes and Actions

### 1) Java generation failed

Typical symptom:
- `run_parity_and_compare_java.sh` fails before compare report generation.

Check in order:
1. DNS preflight (`repo.eclipse.org`, `repo.maven.apache.org`) result in logs.
2. Maven/Tycho repo isolation path (`JAVA_PARITY_MVN_LOCAL_REPO`) value.
3. Retry settings (`java_generate_retries`, `java_generate_retry_delay_secs`).

Actions:
1. Re-run once with higher retries.
2. If network incident is confirmed, keep strict default but temporarily set `java_allow_generate_failure=true` for diagnostic-only runs.
3. Revert `java_allow_generate_failure` back to `false` after incident closes.

### 2) Java parity gate failed (`tests/java_vs_rust.md`)

Typical symptom:
- `check_java_parity.sh` fails on results compare.

Check in order:
1. Rust parity input freshness (`tests/results_layered_issue_scenarios.csv`).
2. Java CSV freshness (`tests/java_results_layered_issue_scenarios.csv`).
3. Threshold (`java_parity_threshold`) and scenario coverage.

Actions:
1. Re-run pipeline once to remove transient noise.
2. If reproducible, investigate Rust regression first.
3. Only adjust threshold with explicit team decision.

### 3) Baseline parity gate failed (`tests/java_vs_rust_baseline.md`)

Typical symptom:
- `check_java_parity.sh` fails in baseline mode.

Actions:
1. Confirm baseline file actually reflects the current approved Java run.
2. If baseline is stale, regenerate Java CSV and promote candidate through strict gate.
3. Do not bypass baseline gate in default branch policy.

### 4) Candidate readiness failed

Typical symptom:
- `tests/java_baseline_candidate_check.md` status is `not_ready` or step failed.

Check in order:
1. Required scenarios exist.
2. Minimum rows satisfy `max(java_artifact_min_rows, scenario_count)`.
3. Parity check status in candidate check report.

Actions:
1. Re-run generation with same scenario set.
2. Fix missing scenario/data quality issue.
3. Promote baseline only when check report is `ready`.

## Promotion Rule

Promote Java baseline only when all are true:
- `tests/java_vs_rust.md` exists and parity passed.
- `tests/java_vs_rust_baseline.md` exists and parity passed.
- `tests/java_baseline_candidate_check.md` is `ready`.
- Candidate reflects the intended scenario set.
