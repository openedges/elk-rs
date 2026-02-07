# Java vs Rust Layered Issue Perf

- rust file: `perf/results_layered_issue_scenarios.csv`
- java file: `perf/java_results_layered_issue_scenarios.csv`
- window: `3`

| scenario | rust_avg_ms | java_avg_ms | avg_delta_vs_java_% | rust_scenarios_per_sec | java_scenarios_per_sec | ops_delta_vs_java_% |
|---|---:|---:|---:|---:|---:|---:|
| issue_405 | 0.903452 | 3.291471 | -72.55 | 1188.370000 | 303.820000 | 291.14 |
| issue_603 | 0.306013 | 0.575340 | -46.81 | 3275.323333 | 1738.100000 | 88.44 |
| issue_680 | 0.361782 | 0.394665 | -8.33 | 2770.190000 | 2533.800000 | 9.33 |
| issue_871 | 0.575931 | 1.471192 | -60.85 | 1753.670000 | 679.720000 | 158.00 |
| issue_905 | 0.467043 | 0.694552 | -32.76 | 2171.646667 | 1439.780000 | 50.83 |

- common scenarios: 5
- rust better/equal on both metrics: 5
- rust slower on both metrics: 0
