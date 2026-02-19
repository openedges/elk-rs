# ELK Model Parity Report

- manifest: `perf/model_parity_step50_pendulum3d_prefetch_scan/p18/rust_manifest.tsv`
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
| coordinate | 13 | 65.0% |
| section | 5 | 25.0% |
| structure | 2 | 10.0% |

### Top Diff Path Prefixes

- `children[*]/children[*]/y`: 7 (35.0%)
- `children[*]/children[*]/edges[*]`: 7 (35.0%)
- `children[*]/children[*]/children[*]`: 4 (20.0%)
- `children[*]/y`: 2 (10.0%)

## Drift Samples

- `realworld/ptolemy/hierarchical/continuous_pendulum3d_Pendulum3D.elkt`: diffs=20 [coordinate=13, section=5, structure=2], first: children[2]/y: number mismatch (411.4 != 113.0)
