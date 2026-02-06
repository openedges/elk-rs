# Java vs Rust Layered Issue Perf

- rust file: `perf/results_layered_issue_scenarios.csv`
- java file: `perf/baselines/java_layered_issue_scenarios.csv`
- window: `1`

| scenario | rust_avg_ms | java_avg_ms | avg_delta_vs_java_% | rust_scenarios_per_sec | java_scenarios_per_sec | ops_delta_vs_java_% |
|---|---:|---:|---:|---:|---:|---:|
| issue_405 | 2.255875 | 3.291471 | -31.46 | 443.290000 | 303.820000 | 45.91 |
| issue_603 | 0.406541 | 0.575340 | -29.34 | 2459.780000 | 1738.100000 | 41.52 |
| issue_680 | 0.411417 | 0.394665 | 4.24 | 2430.620000 | 2533.800000 | -4.07 |
| issue_871 | 0.671000 | 1.471192 | -54.39 | 1490.310000 | 679.720000 | 119.25 |
| issue_905 | 0.620208 | 0.694552 | -10.70 | 1612.360000 | 1439.780000 | 11.99 |

- common scenarios: 5
- rust better/equal on both metrics: 4
- rust slower on both metrics: 1
