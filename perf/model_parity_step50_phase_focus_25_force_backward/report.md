# ELK Model Parity Report

- manifest: `perf/model_parity_step50_phase_focus_25_force_backward/rust_manifest.tsv`
- total rows: 25
- compared rows: 25
- matched rows: 17
- drift rows: 8
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 160

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 100 | 62.5% |
| section | 36 | 22.5% |
| structure | 24 | 15.0% |

### Top Diff Path Prefixes

- `children[*]/children[*]/edges[*]`: 48 (30.0%)
- `children[*]/children[*]/y`: 40 (25.0%)
- `children[*]/y`: 36 (22.5%)
- `children[*]/children[*]/children[*]`: 24 (15.0%)
- `edges[*]`: 4 (2.5%)
- `edges[*]/sections[*]`: 4 (2.5%)
- `edges[*]/sections[*]/endPoint`: 4 (2.5%)

## Drift Samples

- `realworld/ptolemy/hierarchical/continuous_cartpendulum_CartPendulum.elkg`: diffs=20 [coordinate=10, section=6, structure=4], first: children[2]/children[0]/y: number mismatch (272.4 != 254.4)
- `realworld/ptolemy/hierarchical/continuous_cartpendulum_CartPendulum.elkt`: diffs=20 [coordinate=10, section=6, structure=4], first: children[2]/children[0]/y: number mismatch (272.4 != 254.4)
- `realworld/ptolemy/hierarchical/continuous_pendulum3d_Pendulum3D.elkg`: diffs=20 [coordinate=13, section=5, structure=2], first: children[2]/y: number mismatch (411.4 != 459.4)
- `realworld/ptolemy/hierarchical/continuous_pendulum3d_Pendulum3D.elkt`: diffs=20 [coordinate=13, section=5, structure=2], first: children[2]/y: number mismatch (411.4 != 459.4)
- `realworld/ptolemy/hierarchical/ptolemy_execdemos_demos_hyvisualdemos_Pendulum3D.elkg`: diffs=20 [coordinate=13, section=5, structure=2], first: children[2]/y: number mismatch (411.4 != 459.4)
- `realworld/ptolemy/hierarchical/ptolemy_execdemos_demos_hyvisualdemos_Pendulum3D.elkt`: diffs=20 [coordinate=13, section=5, structure=2], first: children[2]/y: number mismatch (411.4 != 459.4)
- `realworld/ptolemy/flattened/ptolemy_brewery_Brewery.elkg`: diffs=20 [coordinate=14, structure=4, section=2], first: children[1]/y: number mismatch (313.0 != 151.0)
- `realworld/ptolemy/flattened/ptolemy_brewery_Brewery.elkt`: diffs=20 [coordinate=14, structure=4, section=2], first: children[1]/y: number mismatch (313.0 != 151.0)
