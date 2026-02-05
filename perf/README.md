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
- `scripts/summarize_perf_results.sh` writes `perf/summary.md` with the latest run and the last 5 runs for each perf test.
- `scripts/check_perf_regression.sh` exits non-zero when avg_ms or ops/elems per sec regress more than a threshold (default 5%), using windowed averages (default window 3; needs 2*window lines).
- `scripts/run_perf_and_check.sh` runs all perf scripts, compares, summarizes, then checks regressions (args: threshold, window).
- `scripts/run_perf_and_compare.sh [window]` runs all perf scripts, compares with the given window, then summarizes.
- `scripts/run_perf_layered_layout.sh` runs recursive layout perf with the layered algorithm (default output `perf/results_recursive_layout_layered.csv`).
