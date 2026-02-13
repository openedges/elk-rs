# ELK Model Parity Report

- manifest: `/Users/cody.ij.hwang/Projects/github/elk-rs/perf/model_parity_full/rust_manifest.tsv`
- total rows: 1448
- compared rows: 1436
- matched rows: 304
- drift rows: 1132
- skipped rows (java/rust non-ok): 12
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 21455

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 14660 | 68.3% |
| section | 5737 | 26.7% |
| structure | 589 | 2.7% |
| label | 222 | 1.0% |
| ordering | 152 | 0.7% |
| other | 95 | 0.4% |

### Top Diff Path Prefixes

- `children[*]/y`: 6069 (28.3%)
- `children[*]/x`: 3074 (14.3%)
- `children[*]/edges[*]/sections[*]`: 2829 (13.2%)
- `children[*]/children[*]/x`: 1659 (7.7%)
- `children[*]/children[*]/y`: 1281 (6.0%)
- `children[*]/children[*]/children[*]`: 1205 (5.6%)
- `children[*]/children[*]/edges[*]`: 955 (4.5%)
- `edges[*]/sections[*]/endPoint`: 745 (3.5%)
- `edges[*]/sections[*]/bendPoints[*]`: 670 (3.1%)
- `edges[*]/sections[*]/startPoint`: 669 (3.1%)

## Drift Samples

- `examples/edges/insideSelfLoops.elkt`: diffs=18 [section=16, coordinate=2], first: children[0]/y: number mismatch (22.0 != 12.0)
- `examples/general/spacing/labels.elkt`: diffs=20 [section=18, coordinate=2], first: children[0]/x: number mismatch (52.0 != 72.0)
- `examples/general/spacing/nodesEdges.elkt`: diffs=20 [section=14, coordinate=6], first: children[0]/y: number mismatch (37.0 != 33.0)
- `examples/general/spacing/ports.elkt`: diffs=1 [coordinate=1], first: children[2]/children[1]/height: number mismatch (60.0 != 40.0)
- `examples/general/spacing/portsSurrounding.elkt`: diffs=6 [coordinate=6], first: children[0]/ports[0]/y: number mismatch (57.0 != 57.5)
- `examples/hierarchy/hierarchicalLayoutMixing.elkt`: diffs=20 [coordinate=9, section=8, structure=3], first: children[0]/children[0]/y: number mismatch (12.0 != 76.0)
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
