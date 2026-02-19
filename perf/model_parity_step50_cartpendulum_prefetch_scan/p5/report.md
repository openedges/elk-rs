# ELK Model Parity Report

- manifest: `perf/model_parity_step50_cartpendulum_prefetch_scan/p5/rust_manifest.tsv`
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
| section | 10 | 50.0% |
| coordinate | 7 | 35.0% |
| structure | 3 | 15.0% |

### Top Diff Path Prefixes

- `children[*]/edges[*]/sections[*]`: 11 (55.0%)
- `children[*]/children[*]/y`: 5 (25.0%)
- `children[*]/y`: 2 (10.0%)
- `children[*]/edges[*]`: 2 (10.0%)

## Drift Samples

- `realworld/ptolemy/hierarchical/continuous_cartpendulum_CartPendulum.elkt`: diffs=20 [section=10, coordinate=7, structure=3], first: children[0]/y: number mismatch (40.16666666666667 != 308.06666666666666)
