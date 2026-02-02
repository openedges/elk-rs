Scripts overview:

- `run_perf_comment_attachment.sh [count] [iterations] [warmup] [output]`
- `run_perf_graph_validation.sh [nodes] [edges] [iterations] [warmup] [mode] [output]`
- `run_perf_recursive_layout.sh [nodes] [edges] [iterations] [warmup] [algorithm] [validate_graph] [validate_options] [output]`
- `run_perf_all.sh` (runs all perf scripts with defaults)
- `compare_perf_results.sh [window]` (windowed compare of last two windows)
- `summarize_perf_results.sh [output]` (writes `perf/summary.md` by default)
- `check_perf_regression.sh [threshold] [window]` (default 5%, window 3)
- `run_perf_and_compare.sh [window]` (perf + compare + summary)
- `run_perf_and_check.sh [threshold] [window]` (perf + compare + summary + regression gate)
- `run_all_checks.sh [threshold] [window]` (cargo test, clippy, perf gate)
- `run_fast_checks.sh` (cargo test, clippy only)
