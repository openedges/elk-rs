# ELK Model Parity Report

- manifest: `perf/model_parity_step50_pendulum3d_prefetch_scan/p22/rust_manifest.tsv`
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
| section | 8 | 40.0% |
| coordinate | 7 | 35.0% |
| structure | 5 | 25.0% |

### Top Diff Path Prefixes

- `children[*]/edges[*]/sections[*]`: 8 (40.0%)
- `children[*]/children[*]/y`: 4 (20.0%)
- `children[*]/y`: 2 (10.0%)
- `children[*]/edges[*]`: 2 (10.0%)
- `edges[*]`: 1 (5.0%)
- `edges[*]/sections[*]/bendPoints[*]`: 1 (5.0%)
- `edges[*]/sections[*]/endPoint`: 1 (5.0%)
- `edges[*]/junctionPoints[*]/y`: 1 (5.0%)

## Drift Samples

- `realworld/ptolemy/hierarchical/continuous_pendulum3d_Pendulum3D.elkt`: diffs=20 [section=8, coordinate=7, structure=5], first: children[2]/y: number mismatch (411.4 != 113.0)
