# ELK Model Parity Report

- manifest: `perf/model_parity_step50_pendulum3d_prefetch_scan/p7/rust_manifest.tsv`
- total rows: 1
- compared rows: 1
- matched rows: 0
- drift rows: 1
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 13

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| section | 8 | 61.5% |
| coordinate | 3 | 23.1% |
| structure | 2 | 15.4% |

### Top Diff Path Prefixes

- `edges[*]/sections[*]/bendPoints[*]`: 4 (30.8%)
- `edges[*]/sections[*]/endPoint`: 4 (30.8%)
- `children[*]/y`: 2 (15.4%)
- `edges[*]`: 2 (15.4%)
- `edges[*]/junctionPoints[*]/y`: 1 (7.7%)

## Drift Samples

- `realworld/ptolemy/hierarchical/continuous_pendulum3d_Pendulum3D.elkt`: diffs=13 [section=8, coordinate=3, structure=2], first: children[2]/y: number mismatch (411.4 != 113.0)
