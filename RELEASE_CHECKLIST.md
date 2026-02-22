# Release Checklist

Before release, pass the following gates in order to mark the build as release-ready.

## 0) Validation Flow (Run in This Order)

Run the core validation flow in this exact order:

1. `LAYERED_PHASE_WIRING_PARITY_STRICT=true sh scripts/check_layered_phase_wiring_parity.sh`
2. `cargo build --workspace`
3. `cargo clippy --workspace --all-targets`
4. `cargo test --workspace`
5. `MODEL_PARITY_SKIP_JAVA_EXPORT=true sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models perf/model_parity_full`
6. `cargo build --workspace --release`

Pass criteria for each step:

- Step 1: `perf/layered_phase_wiring_parity.md` reports `status: ok`
- Step 2: build error/warning count is zero
- Step 3: clippy warning count is zero
- Step 4: test failure count is zero
- Step 5: parity report is generated and drift metrics are reviewed/recorded
- Step 6: release profile build succeeds

If a step fails, apply this triage loop:

1. Reproduce with a focused command (single crate/test/script).
2. Narrow scope to the failing module/phase.
3. If needed, run Java/Rust phase-trace comparison for divergence localization.
4. Record root-cause hypothesis and reproduce command in `HISTORY.md`.

## 1) Code Quality Gates (Required)

```sh
cargo build --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
cargo build --workspace --release
```

Decision:
- If any command fails, stop the release.

## 2) Java Parity / Metadata Parity Gates (Required)

```sh
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
LAYERED_PHASE_WIRING_PARITY_STRICT=true sh scripts/check_layered_phase_wiring_parity.sh
```

Decision:
- If any report (`perf/*parity.md`) is not `status: ok`, stop the release.

## 3) Performance Gates (Required + Recommended)

Required checks:

```sh
PERF_COMPARE_MODE=baseline sh scripts/check_perf_regression.sh 5 3
sh scripts/check_recursive_perf_runtime_budget.sh perf/results_recursive_layout_scenarios.csv default perf/recursive_runtime_budget.md
```

Java comparison checks:

```sh
sh scripts/check_java_perf_parity.sh perf/results_layered_issue_scenarios.csv perf/java_results_layered_issue_scenarios.csv 3 0
sh scripts/check_java_perf_parity_scenarios.sh perf/results_layered_issue_scenarios.csv perf/java_results_layered_issue_scenarios.csv 3 perf/java_parity_thresholds.csv
```

Decision:
- A failure in `check_perf_regression.sh 5 3` means "current Rust is >5% slower than Rust baseline."
- This is different from Java-vs-Rust parity.
- If Java parity checks pass, it does not mean Rust is slower than Java.

## 4) Release Decision Rules

- `code quality + parity + runtime budget` all pass: release allowed.
- Only `baseline 5% regression` fails and Java parity passes:
  - default policy: investigate the regression before release.
  - exception: emergency release is allowed, but document the perf risk in release notes.
- Java parity fails: stop the release.

## 5) Artifacts to Review (Recommended)

- `perf/core_options_parity.md`
- `perf/core_option_dependency_parity.md`
- `perf/algorithm_*_parity.md`
- `perf/layered_phase_wiring_parity.md`
- `perf/recursive_runtime_budget.md`
- `perf/java_vs_rust.md`
- `perf/java_vs_rust_baseline.md`
- `perf/java_perf_status.md`

## 6) When Baseline Updates Are Needed

Update baseline only when all conditions are met:
- there is an intentional performance change (algorithm/options/environment),
- repeated measurements show stable variance,
- the team approves baseline movement.

Procedure:

```sh
sh scripts/run_perf_layered_issue_scenarios.sh "issue_405,issue_603,issue_680,issue_871,issue_905" 20 3 perf/results_layered_issue_scenarios.csv
PERF_RECURSIVE_SCENARIO_PROFILE=default sh scripts/run_perf_recursive_layout_scenarios.sh "" 5 1 perf/results_recursive_layout_scenarios.csv
sh scripts/update_perf_baseline.sh
sh scripts/update_perf_recursive_scenarios_baseline.sh
PERF_COMPARE_MODE=baseline sh scripts/check_perf_regression.sh 5 3
```

Follow detailed baseline operations in `perf/baselines/POLICY.md`.
