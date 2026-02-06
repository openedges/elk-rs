# Layered Issue Regression Analysis

## Scope

- Target scenarios: `issue_405`, `issue_603`, `issue_680`, `issue_871` (and `issue_905` as control)
- Objective:
  - Validate whether baseline update is justified.
  - Identify root causes behind prior `baseline 5%` regression failures.

## Key Findings

1. Prior `baseline 5%` failures were primarily caused by mixed-run contamination in current CSV data.
   - `perf/results_layered_issue_scenarios.csv` contained mixed configurations for the same scenario (`iterations=1,warmup=0` and `iterations=20,warmup=3`).
   - Existing regression logic consumed the latest `window` rows per scenario without config filtering, which produced false positives.
2. `perf/baselines/layered_issue_scenarios.csv` is low-confidence baseline input.
   - It contains one row per scenario with `iterations=3,warmup=1`.
   - This is below policy-quality guidance for baseline refresh.
3. After filtering current rows by scenario + latest run configuration, baseline gate no longer reports layered regressions.
4. `issue_871` option-path ablation shows model-order/feedback setup is not the slowdown source.
   - `issue_871` (model-order/feedback setup) is faster than `issue_871_plain` in repeated runs.

## Actions Taken

- Updated scripts to avoid mixed-run contamination:
  - `scripts/check_perf_regression.sh`
  - `scripts/compare_perf_results.sh`
- New behavior:
  - For scenario-based files, current-side window calculations use rows matching the latest config tuple for that scenario.
    - `layered_issue_scenarios`: config tuple = `(iterations, warmup)` from columns `(3,4)`
    - `recursive_layout_scenarios`: config tuple = `(iterations, warmup)` from columns `(6,7)`
  - Baseline-side aggregation remains scenario-level (all baseline rows for that scenario), preserving existing baseline compatibility.

## Repeated Measurement Summary (Layered, 20/3)

Measurement file: `/tmp/layered_repeat_20_3.csv`  
Runs: 12 repeated executions (`iterations=20`, `warmup=3`) for all 5 scenarios.

| scenario | n | mean avg_ms | std avg_ms | mean ops/s | std ops/s |
|---|---:|---:|---:|---:|---:|
| issue_405 | 12 | 0.860876 | 0.082569 | 1171.92 | 107.80 |
| issue_603 | 12 | 0.314931 | 0.016635 | 3184.85 | 181.11 |
| issue_680 | 12 | 0.370255 | 0.009846 | 2702.76 | 72.14 |
| issue_871 | 12 | 0.637810 | 0.025697 | 1570.45 | 64.30 |
| issue_905 | 12 | 0.402925 | 0.036938 | 2502.63 | 227.83 |

Against current baseline (`perf/baselines/layered_issue_scenarios.csv`):
- `issue_603`, `issue_680`: within small drift range.
- `issue_871`: small delta on avg_ms (around +6%), near threshold boundary.
- `issue_405`, `issue_905`: baseline appears stale/noisy relative to repeated measurements.

## issue_871 Ablation (Root-Cause Probe)

Added diagnostic scenario `issue_871_plain` to `perf_layered_issue_scenarios` for A/B probing.

Measurement file: `/tmp/issue_871_ablation_repeats.csv`  
Runs: 12 repeated executions (`iterations=80`, `warmup=10`) for `issue_871` and `issue_871_plain`.

| scenario | n | mean avg_ms | std avg_ms | mean ops/s | std ops/s |
|---|---:|---:|---:|---:|---:|
| issue_871 | 12 | 0.624872 | 0.015031 | 1601.30 | 40.35 |
| issue_871_plain | 12 | 0.818957 | 0.022264 | 1221.99 | 33.92 |

Interpretation:
- The model-order/feedback configuration in `issue_871` is not the regression source.
- The plain variant is significantly slower, so disabling those options is not a fix direction.

## Current Gate Status (after script fix + same latest config runs)

- `PERF_COMPARE_MODE=baseline sh scripts/check_perf_regression.sh 5 3`: pass for layered scenarios.
- `sh scripts/check_recursive_perf_runtime_budget.sh ...`: `status: ok`.
- Java parity gates (`check_java_perf_parity.sh`, `check_java_perf_parity_scenarios.sh`): pass.

## Recommendation

1. Keep the script-level config-filter fix (already applied) to prevent false regressions from mixed sample windows.
2. Refresh layered baseline using policy-compliant collection:
   - use stable run config (`20/3` or stricter),
   - collect repeated runs,
   - refresh `perf/baselines/layered_issue_scenarios.csv` with reviewed data.
3. Treat `issue_871` as monitor-only for now (no immediate algorithm rollback needed), since current drift is small and option-ablation does not indicate a targeted regression path.
