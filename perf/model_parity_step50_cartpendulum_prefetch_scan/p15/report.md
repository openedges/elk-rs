# ELK Model Parity Report

- manifest: `perf/model_parity_step50_cartpendulum_prefetch_scan/p15/rust_manifest.tsv`
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
| coordinate | 10 | 50.0% |
| section | 6 | 30.0% |
| structure | 4 | 20.0% |

### Top Diff Path Prefixes

- `children[*]/children[*]/edges[*]`: 10 (50.0%)
- `children[*]/children[*]/y`: 6 (30.0%)
- `children[*]/children[*]/children[*]`: 4 (20.0%)

## Drift Samples

- `realworld/ptolemy/hierarchical/continuous_cartpendulum_CartPendulum.elkt`: diffs=20 [coordinate=10, section=6, structure=4], first: children[2]/children[0]/y: number mismatch (272.4 != 120.0)
