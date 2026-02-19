# ELK Model Parity Report

- manifest: `perf/model_parity_step50_pendulum3d_prefetch_scan/p11/rust_manifest.tsv`
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

- `children[*]/edges[*]/sections[*]`: 8 (57.1%)
- `children[*]/children[*]/y`: 4 (28.6%)
- `children[*]/edges[*]`: 2 (14.3%)

## Drift Samples

- `realworld/ptolemy/hierarchical/continuous_pendulum3d_Pendulum3D.elkt`: diffs=14 [section=6, coordinate=4, structure=4], first: children[4]/children[3]/y: number mismatch (126.4 != 46.400000000000006)
