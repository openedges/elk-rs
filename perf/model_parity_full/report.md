# ELK Model Parity Report

- manifest: `/tmp/rust_manifest_full_v4.tsv`
- total rows: 1448
- compared rows: 1395
- matched rows: 156
- drift rows: 1239
- skipped rows (java/rust non-ok): 53
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 23263

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 16598 | 71.3% |
| section | 3940 | 16.9% |
| structure | 2058 | 8.8% |
| label | 380 | 1.6% |
| ordering | 147 | 0.6% |
| other | 140 | 0.6% |

### Top Diff Path Prefixes

- `children[*]/y`: 7480 (32.2%)
- `children[*]/x`: 3109 (13.4%)
- `children[*]/children[*]/y`: 1635 (7.0%)
- `children[*]/edges[*]/sections[*]`: 1389 (6.0%)
- `children[*]/children[*]/x`: 1168 (5.0%)
- `children[*]/children[*]/children[*]`: 1098 (4.7%)
- `children[*]/ports[*]/y`: 1098 (4.7%)
- `children[*]/edges[*]`: 989 (4.3%)
- `edges[*]/sections[*]/endPoint`: 975 (4.2%)
- `edges[*]/sections[*]/startPoint`: 714 (3.1%)

## Drift Samples

- `examples/edges/insideSelfLoops.elkt`: diffs=18 [section=16, coordinate=2], first: children[0]/y: number mismatch (22.0 != 12.0)
- `examples/hierarchy/hierarchicalEdges.elkt`: diffs=9 [coordinate=6, other=2, structure=1], first: children[0]/children[0]/x: number mismatch (17.0 != 12.0)
- `examples/hierarchy/hierarchicalLayoutMixing.elkt`: diffs=20 [coordinate=10, section=8, structure=2], first: children[0]/children[0]/x: number mismatch (17.0 != 22.558860981188595)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_hierarchyDirection.elkt`: diffs=10 [section=8, coordinate=1, other=1], first: children[3]/y: number mismatch (34.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_motor.elkt`: diffs=20 [section=11, coordinate=6, structure=3], first: children[0]/y: number mismatch (43.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_simpleDirectionTest.elkt`: diffs=18 [section=10, coordinate=4, structure=3, other=1], first: children[0]/x: number mismatch (52.0 != 62.0)
- `examples/user-hints/layered/partitioning.elkt`: diffs=20 [section=11, coordinate=8, structure=1], first: children[0]/children[1]/x: number mismatch (178.0 != 90.0)
- `examples/user-hints/layered/verticalOrder.elkt`: diffs=9 [coordinate=4, structure=2, section=2, label=1], first: children[1]/children[1]/y: number mismatch (52.0 != 32.0)
- `examples/user-hints/model-order/modelOrderCycleBreaking.elkt`: diffs=20 [section=10, coordinate=8, structure=2], first: children[0]/children[0]/y: number mismatch (23.0 != 12.0)
- `examples/user-hints/model-order/modelOrderNoCrossingMin.elkt`: diffs=2 [ordering=2], first: edges[2]/sections[0]/bendPoints: array length mismatch (2 != 4)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTank.elkg`: diffs=20 [coordinate=14, section=6], first: children[0]/y: number mismatch (32.0 != 42.0)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTank.elkt`: diffs=20 [coordinate=14, section=6], first: children[0]/y: number mismatch (32.0 != 42.0)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkg`: diffs=20 [section=11, coordinate=7, ordering=2], first: children[1]/y: number mismatch (203.5 != 289.5)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkt`: diffs=20 [section=11, coordinate=7, ordering=2], first: children[1]/y: number mismatch (203.5 != 289.5)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (653.0 != 657.0)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (653.0 != 657.0)
- `realworld/ptolemy/flattened/aspect_cartrackingattackmodeling_CarTrackingAttackModeling.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1343.0 != 553.0)
- `realworld/ptolemy/flattened/aspect_cartrackingattackmodeling_CarTrackingAttackModeling.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1343.0 != 553.0)
- `realworld/ptolemy/flattened/aspect_cartrackingattackmodeling_CarTrackingAttackModelingAllAttacksInOneModel.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1559.0 != 1639.0)
- `realworld/ptolemy/flattened/aspect_cartrackingattackmodeling_CarTrackingAttackModelingAllAttacksInOneModel.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1559.0 != 1639.0)
