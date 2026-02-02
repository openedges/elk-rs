Scripts overview:

- `run_perf_comment_attachment.sh [count] [iterations] [warmup] [output]`
- `run_perf_graph_validation.sh [nodes] [edges] [iterations] [warmup] [mode] [output]`
- `run_perf_recursive_layout.sh [nodes] [edges] [iterations] [warmup] [algorithm] [validate_graph] [validate_options] [output]`
- `run_perf_all.sh` (runs all perf scripts with defaults; supports env overrides)
- `compare_perf_results.sh [window]` (windowed compare of last two windows)
- `summarize_perf_results.sh [output]` (writes `perf/summary.md` by default)
- `check_perf_regression.sh [threshold] [window]` (default 5%, window 3)
- `run_perf_and_compare.sh [window]` (perf + compare + summary)
- `run_perf_and_check.sh [threshold] [window]` (perf + compare + summary + regression gate)
- `run_all_checks.sh [threshold] [window]` (cargo test, clippy, perf gate)
- `run_fast_checks.sh` (cargo test, clippy only)

`run_perf_all.sh` env overrides (defaults shown):

```
COMMENT_COUNT=2000
COMMENT_ITERATIONS=5
COMMENT_WARMUP=1
COMMENT_OUTPUT=perf/results_comment_attachment.csv
GRAPH_NODES=1000
GRAPH_EDGES=2000
GRAPH_ITERATIONS=5
GRAPH_WARMUP=1
GRAPH_MODE=both
GRAPH_OUTPUT=perf/results_graph_validation.csv
LAYOUT_NODES=500
LAYOUT_EDGES=1000
LAYOUT_ITERATIONS=5
LAYOUT_WARMUP=1
LAYOUT_ALGORITHM=fixed
LAYOUT_VALIDATE_GRAPH=false
LAYOUT_VALIDATE_OPTIONS=false
LAYOUT_OUTPUT=perf/results_recursive_layout.csv
```

CI workflows (GitHub Actions):
- `.github/workflows/ci.yml` runs `run_fast_checks.sh` on push/PR.
- `.github/workflows/perf.yml` runs perf scripts on manual dispatch and uploads CSV/summary artifacts.
