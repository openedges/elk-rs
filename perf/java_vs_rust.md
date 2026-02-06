# Java vs Rust Layered Issue Perf

- rust file: `perf/results_layered_issue_scenarios.csv`
- java file: `perf/java_results_layered_issue_scenarios.csv`
- window: `3`

| scenario | rust_avg_ms | java_avg_ms | avg_delta_vs_java_% | rust_scenarios_per_sec | java_scenarios_per_sec | ops_delta_vs_java_% |
|---|---:|---:|---:|---:|---:|---:|
| issue_405 | 0.979998 | 3.291471 | -70.23 | 1059.160000 | 303.820000 | 248.61 |
| issue_603 | 0.308956 | 0.575340 | -46.30 | 3237.346667 | 1738.100000 | 86.26 |
| issue_680 | 0.349462 | 0.394665 | -11.45 | 2862.610000 | 2533.800000 | 12.98 |
| issue_871 | 0.616492 | 1.471192 | -58.10 | 1625.366667 | 679.720000 | 139.12 |
| issue_905 | 0.445854 | 0.694552 | -35.81 | 2291.920000 | 1439.780000 | 59.19 |

- common scenarios: 5
- rust better/equal on both metrics: 5
- rust slower on both metrics: 0
