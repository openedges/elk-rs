# ELK Model Parity Report

- manifest: `perf/model_parity_step50_pendulum3d_prefetch_scan/p0/rust_manifest.tsv`
- total rows: 1
- compared rows: 1
- matched rows: 0
- drift rows: 1
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 14

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| section | 6 | 42.9% |
| coordinate | 4 | 28.6% |
| structure | 4 | 28.6% |

### Top Diff Path Prefixes

- `children[*]/children[*]/edges[*]`: 10 (71.4%)
- `children[*]/children[*]/children[*]`: 4 (28.6%)

## Drift Samples

- `realworld/ptolemy/hierarchical/continuous_pendulum3d_Pendulum3D.elkt`: diffs=14 [section=6, coordinate=4, structure=4], first: children[4]/children[7]/children[0]/y: number mismatch (46.4 != 126.4)
