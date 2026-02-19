# ELK Model Parity Report

- manifest: `perf/model_parity_step50_phase_focus_25_after_graph_order/rust_manifest.tsv`
- total rows: 25
- compared rows: 25
- matched rows: 19
- drift rows: 6
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 84

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| section | 36 | 42.9% |
| coordinate | 24 | 28.6% |
| structure | 24 | 28.6% |

### Top Diff Path Prefixes

- `children[*]/children[*]/edges[*]`: 60 (71.4%)
- `children[*]/children[*]/children[*]`: 24 (28.6%)

## Drift Samples

- `realworld/ptolemy/hierarchical/continuous_cartpendulum_CartPendulum.elkg`: diffs=14 [section=6, coordinate=4, structure=4], first: children[2]/children[8]/children[0]/y: number mismatch (32.0 != 112.0)
- `realworld/ptolemy/hierarchical/continuous_cartpendulum_CartPendulum.elkt`: diffs=14 [section=6, coordinate=4, structure=4], first: children[2]/children[8]/children[0]/y: number mismatch (32.0 != 112.0)
- `realworld/ptolemy/hierarchical/continuous_pendulum3d_Pendulum3D.elkg`: diffs=14 [section=6, coordinate=4, structure=4], first: children[4]/children[7]/children[0]/y: number mismatch (46.4 != 126.4)
- `realworld/ptolemy/hierarchical/continuous_pendulum3d_Pendulum3D.elkt`: diffs=14 [section=6, coordinate=4, structure=4], first: children[4]/children[7]/children[0]/y: number mismatch (46.4 != 126.4)
- `realworld/ptolemy/hierarchical/ptolemy_execdemos_demos_hyvisualdemos_Pendulum3D.elkg`: diffs=14 [section=6, coordinate=4, structure=4], first: children[4]/children[7]/children[0]/y: number mismatch (46.4 != 126.4)
- `realworld/ptolemy/hierarchical/ptolemy_execdemos_demos_hyvisualdemos_Pendulum3D.elkt`: diffs=14 [section=6, coordinate=4, structure=4], first: children[4]/children[7]/children[0]/y: number mismatch (46.4 != 126.4)
