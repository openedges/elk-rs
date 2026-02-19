# ELK Model Parity Report

- manifest: `perf/model_parity_step50_cartpendulum_prefetch_scan/p30/rust_manifest.tsv`
- total rows: 1
- compared rows: 1
- matched rows: 0
- drift rows: 1
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 20

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 12 | 60.0% |
| section | 5 | 25.0% |
| structure | 3 | 15.0% |

### Top Diff Path Prefixes

- `children[*]/children[*]/edges[*]`: 8 (40.0%)
- `children[*]/children[*]/y`: 6 (30.0%)
- `children[*]/children[*]/children[*]`: 4 (20.0%)
- `children[*]/y`: 2 (10.0%)

## Drift Samples

- `realworld/ptolemy/hierarchical/continuous_cartpendulum_CartPendulum.elkt`: diffs=20 [coordinate=12, section=5, structure=3], first: children[0]/y: number mismatch (40.16666666666667 != 388.06666666666666)
