# ELK Model Parity Report

- manifest: `perf/model_parity_full/rust_manifest.tsv`
- total rows: 1448
- compared rows: 1439
- matched rows: 169
- drift rows: 1270
- skipped rows (java/rust non-ok): 9
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 23793

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 16806 | 70.6% |
| section | 4149 | 17.4% |
| structure | 2129 | 8.9% |
| label | 407 | 1.7% |
| other | 151 | 0.6% |
| ordering | 151 | 0.6% |

### Top Diff Path Prefixes

- `children[*]/y`: 7553 (31.7%)
- `children[*]/x`: 3134 (13.2%)
- `children[*]/children[*]/y`: 1657 (7.0%)
- `children[*]/edges[*]/sections[*]`: 1433 (6.0%)
- `children[*]/children[*]/x`: 1189 (5.0%)
- `children[*]/children[*]/children[*]`: 1160 (4.9%)
- `children[*]/ports[*]/y`: 1074 (4.5%)
- `edges[*]/sections[*]/endPoint`: 1021 (4.3%)
- `children[*]/edges[*]`: 1012 (4.3%)
- `edges[*]/sections[*]/bendPoints[*]`: 775 (3.3%)

## Drift Samples

- `examples/edges/insideSelfLoops.elkt`: diffs=18 [section=16, coordinate=2], first: children[0]/y: number mismatch (22.0 != 12.0)
- `examples/hierarchy/hierarchicalEdges.elkt`: diffs=9 [coordinate=6, other=2, structure=1], first: children[0]/children[0]/x: number mismatch (17.0 != 12.0)
- `examples/hierarchy/hierarchicalLayoutMixing.elkt`: diffs=20 [coordinate=10, section=8, structure=2], first: children[0]/children[0]/x: number mismatch (17.0 != 22.558860981188595)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_motor.elkt`: diffs=20 [section=11, coordinate=6, structure=3], first: children[0]/y: number mismatch (43.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_motor_pseudo_positions.elkt`: diffs=20 [section=15, coordinate=4, structure=1], first: children[0]/y: number mismatch (23.0 != 34.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_simpleDirectionTest.elkt`: diffs=18 [section=10, coordinate=4, structure=3, other=1], first: children[0]/x: number mismatch (52.0 != 62.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_simpleDirectionTest_pseudo_positions.elkt`: diffs=18 [section=10, coordinate=4, structure=3, other=1], first: children[0]/x: number mismatch (52.0 != 62.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_snake_pseudo_positions.elkt`: diffs=8 [section=4, coordinate=2, structure=2], first: children[1]/y: number mismatch (98.0 != 87.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_sortingTask_pseudo_positions.elkt`: diffs=14 [section=10, coordinate=3, other=1], first: children[2]/y: number mismatch (53.0 != 52.0)
- `examples/user-hints/layered/horizontalOrder.elkt`: diffs=13 [coordinate=6, section=5, structure=1, other=1], first: children[1]/y: number mismatch (151.0 != 140.0)
- `examples/user-hints/layered/partitioning.elkt`: diffs=20 [section=11, coordinate=8, structure=1], first: children[0]/children[1]/x: number mismatch (178.0 != 90.0)
- `examples/user-hints/layered/verticalOrder.elkt`: diffs=9 [coordinate=4, structure=2, section=2, label=1], first: children[1]/children[1]/y: number mismatch (52.0 != 32.0)
- `examples/user-hints/model-order/modelOrderCrossingMinimization.elkt`: diffs=20 [label=8, coordinate=7, section=5], first: children[0]/children[1]/x: number mismatch (82.0 != 62.0)
- `examples/user-hints/model-order/modelOrderCycleBreaking.elkt`: diffs=20 [section=10, coordinate=8, structure=2], first: children[0]/children[0]/y: number mismatch (23.0 != 12.0)
- `examples/user-hints/model-order/modelOrderNoCrossingMin.elkt`: diffs=2 [ordering=2], first: edges[2]/sections[0]/bendPoints: array length mismatch (2 != 4)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTank.elkg`: diffs=20 [coordinate=14, section=6], first: children[0]/y: number mismatch (32.0 != 42.0)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTank.elkt`: diffs=20 [coordinate=14, section=6], first: children[0]/y: number mismatch (32.0 != 42.0)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkg`: diffs=20 [section=11, coordinate=7, ordering=2], first: children[1]/y: number mismatch (203.5 != 289.5)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkt`: diffs=20 [section=11, coordinate=7, ordering=2], first: children[1]/y: number mismatch (203.5 != 289.5)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (653.0 != 657.0)
