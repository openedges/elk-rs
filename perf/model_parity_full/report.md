# ELK Model Parity Report

- manifest: `/Users/cody.ij.hwang/Projects/github/elk-rs/perf/model_parity_full/rust_manifest.tsv`
- total rows: 1448
- compared rows: 1439
- matched rows: 293
- drift rows: 1146
- skipped rows (java/rust non-ok): 9
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 21360

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 15284 | 71.6% |
| section | 3666 | 17.2% |
| structure | 1917 | 9.0% |
| label | 224 | 1.0% |
| ordering | 148 | 0.7% |
| other | 121 | 0.6% |

### Top Diff Path Prefixes

- `children[*]/y`: 6171 (28.9%)
- `children[*]/x`: 3028 (14.2%)
- `children[*]/children[*]/y`: 1552 (7.3%)
- `children[*]/edges[*]/sections[*]`: 1405 (6.6%)
- `children[*]/children[*]/x`: 1183 (5.5%)
- `children[*]/children[*]/children[*]`: 1134 (5.3%)
- `children[*]/ports[*]/y`: 1109 (5.2%)
- `children[*]/edges[*]`: 1012 (4.7%)
- `edges[*]/sections[*]/endPoint`: 780 (3.7%)
- `edges[*]/sections[*]/startPoint`: 709 (3.3%)

## Drift Samples

- `examples/general/spacing/labels.elkt`: diffs=20 [section=18, coordinate=2], first: children[0]/x: number mismatch (52.0 != 72.0)
- `examples/general/spacing/nodesEdges.elkt`: diffs=20 [section=14, coordinate=6], first: children[0]/y: number mismatch (37.0 != 33.0)
- `examples/general/spacing/ports.elkt`: diffs=1 [coordinate=1], first: children[2]/children[1]/height: number mismatch (60.0 != 40.0)
- `examples/hierarchy/hierarchicalEdges.elkt`: diffs=5 [coordinate=3, section=1, other=1], first: children[0]/children[0]/x: number mismatch (17.0 != 12.0)
- `examples/hierarchy/hierarchicalLayoutMixing.elkt`: diffs=20 [coordinate=10, section=8, structure=2], first: children[0]/children[0]/x: number mismatch (17.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_circle.elkt`: diffs=6 [structure=3, section=2, coordinate=1], first: children[1]/y: number mismatch (23.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_circle_pseudo_positions.elkt`: diffs=6 [structure=3, section=2, coordinate=1], first: children[0]/y: number mismatch (12.0 != 23.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_hierarchyDirection.elkt`: diffs=20 [section=11, coordinate=6, structure=3], first: children[0]/y: number mismatch (67.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_hierarchyDirection_pseudo_positions.elkt`: diffs=20 [section=11, coordinate=6, structure=3], first: children[0]/y: number mismatch (57.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_motor_pseudo_positions.elkt`: diffs=20 [coordinate=10, section=8, structure=2], first: children[0]/x: number mismatch (82.0 != 72.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_sortingTask.elkt`: diffs=6 [coordinate=2, structure=2, section=2], first: children[5]/y: number mismatch (32.0 != 12.0)
- `examples/user-hints/layered/horizontalOrder.elkt`: diffs=12 [coordinate=4, structure=4, section=4], first: children[1]/children[0]/y: number mismatch (43.0 != 32.0)
- `examples/user-hints/layered/partitioning.elkt`: diffs=20 [coordinate=11, section=8, structure=1], first: children[0]/children[0]/x: number mismatch (24.0 != 201.0)
- `examples/user-hints/layered/reverseEdge.elkt`: diffs=12 [structure=6, section=4, coordinate=2], first: children[0]/children[2]/y: number mismatch (12.0 != 23.0)
- `examples/user-hints/layered/verticalOrder.elkt`: diffs=9 [coordinate=4, structure=2, section=2, label=1], first: children[1]/children[1]/y: number mismatch (52.0 != 32.0)
- `examples/user-hints/model-order/modelOrderCrossingMinimization.elkt`: diffs=20 [section=12, label=4, coordinate=3, ordering=1], first: children[0]/edges[1]/labels[0]/y: number mismatch (85.0 != 30.0)
- `examples/user-hints/model-order/modelOrderCycleBreaking.elkt`: diffs=20 [section=12, coordinate=7, structure=1], first: children[0]/children[0]/x: number mismatch (62.0 != 82.0)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTank.elkg`: diffs=20 [coordinate=14, section=4, structure=1, ordering=1], first: children[5]/x: number mismatch (251.0 != 513.0)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTank.elkt`: diffs=20 [coordinate=14, section=4, structure=1, ordering=1], first: children[5]/x: number mismatch (251.0 != 513.0)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkg`: diffs=20 [coordinate=16, section=2, ordering=2], first: children[0]/y: number mismatch (96.5 != 358.5)
