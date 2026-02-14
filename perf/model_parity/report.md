# ELK Model Parity Report

- manifest: `/Users/luuvish/Projects/research/elk-rs/perf/model_parity/rust_manifest.tsv`
- total rows: 1448
- compared rows: 1439
- matched rows: 658
- drift rows: 781
- skipped rows (java/rust non-ok): 9
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 14676

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 9162 | 62.4% |
| section | 4820 | 32.8% |
| structure | 249 | 1.7% |
| label | 243 | 1.7% |
| ordering | 109 | 0.7% |
| other | 93 | 0.6% |

### Top Diff Path Prefixes

- `children[*]/edges[*]/sections[*]`: 2774 (18.9%)
- `children[*]/y`: 2263 (15.4%)
- `children[*]/children[*]/x`: 1819 (12.4%)
- `children[*]/x`: 1459 (9.9%)
- `children[*]/children[*]/y`: 1108 (7.5%)
- `children[*]/children[*]/children[*]`: 1099 (7.5%)
- `children[*]/children[*]/edges[*]`: 901 (6.1%)
- `edges[*]/sections[*]/bendPoints[*]`: 467 (3.2%)
- `children[*]/ports[*]/y`: 428 (2.9%)
- `edges[*]/sections[*]/endPoint`: 404 (2.8%)

## Drift Samples

- `examples/edges/insideSelfLoops.elkt`: diffs=6 [section=4, coordinate=1, other=1], first: children[1]/ports[1]/x: number mismatch (100.0 != 200.0)
- `examples/general/spacing/labels.elkt`: diffs=20 [section=18, coordinate=2], first: children[0]/x: number mismatch (52.0 != 72.0)
- `examples/general/spacing/nodesEdges.elkt`: diffs=20 [section=14, coordinate=6], first: children[0]/y: number mismatch (37.0 != 33.0)
- `examples/general/spacing/ports.elkt`: diffs=5 [coordinate=5], first: children[0]/ports[0]/x: number mismatch (16.666666666666668 != 33.333333333333336)
- `examples/general/spacing/portsSurrounding.elkt`: diffs=10 [coordinate=10], first: children[0]/ports[0]/y: number mismatch (57.0 != 57.5)
- `examples/hierarchy/hierarchicalLayoutMixing.elkt`: diffs=20 [coordinate=9, section=8, structure=3], first: children[0]/children[0]/y: number mismatch (12.0 != 76.0)
- `examples/labels/portLabelsMulti.elkt`: diffs=8 [coordinate=4, label=4], first: children[0]/ports[2]/y: number mismatch (120.0 != 60.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_hierarchyDirection.elkt`: diffs=20 [section=14, coordinate=4, structure=1, other=1], first: children[0]/y: number mismatch (67.0 != 76.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_hierarchyDirection_pseudo_positions.elkt`: diffs=20 [section=14, coordinate=4, structure=1, other=1], first: children[0]/y: number mismatch (57.0 != 66.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_motor_pseudo_positions.elkt`: diffs=20 [coordinate=10, section=8, structure=2], first: children[0]/x: number mismatch (82.0 != 72.0)
- `examples/user-hints/layered/partitioning.elkt`: diffs=20 [coordinate=11, section=9], first: children[0]/children[0]/x: number mismatch (24.0 != 201.0)
- `examples/user-hints/layered/verticalOrder.elkt`: diffs=9 [coordinate=4, structure=2, section=2, label=1], first: children[1]/children[1]/y: number mismatch (52.0 != 32.0)
- `examples/user-hints/model-order/modelOrderCrossingMinimization.elkt`: diffs=20 [label=8, section=7, coordinate=3, ordering=2], first: children[0]/edges[1]/labels[0]/y: number mismatch (85.0 != 30.0)
- `examples/user-hints/model-order/modelOrderCycleBreaking.elkt`: diffs=20 [section=12, coordinate=7, structure=1], first: children[0]/children[0]/x: number mismatch (62.0 != 82.0)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (653.0 != 610.0)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (653.0 != 610.0)
- `realworld/ptolemy/flattened/backtrack_primetest_PrimeTest.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1170.0 != 206.0)
- `realworld/ptolemy/flattened/backtrack_primetest_PrimeTest.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1170.0 != 206.0)
- `realworld/ptolemy/flattened/continuous_cartracking_CarTracking.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1762.0 != 1754.0)
- `realworld/ptolemy/flattened/continuous_cartracking_CarTracking.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1762.0 != 1754.0)
