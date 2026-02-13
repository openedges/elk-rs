# ELK Model Parity Report

- manifest: `/Users/cody.ij.hwang/Projects/github/elk-rs/perf/model_parity_realworld/rust_manifest.tsv`
- total rows: 50
- compared rows: 50
- matched rows: 12
- drift rows: 38
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 734

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 632 | 86.1% |
| section | 70 | 9.5% |
| structure | 26 | 3.5% |
| ordering | 6 | 0.8% |

### Top Diff Path Prefixes

- `children[*]/y`: 418 (56.9%)
- `children[*]/x`: 206 (28.1%)
- `edges[*]/sections[*]/endPoint`: 28 (3.8%)
- `edges[*]/sections[*]/startPoint`: 26 (3.5%)
- `edges[*]/sections[*]`: 16 (2.2%)
- `edges[*]/sections[*]/bendPoints[*]`: 16 (2.2%)
- `edges[*]`: 10 (1.4%)
- `edges[*]/junctionPoints[*]/y`: 6 (0.8%)
- `edges[*]/sections[*]/bendPoints`: 4 (0.5%)
- `edges[*]/junctionPoints[*]/x`: 2 (0.3%)

## Drift Samples

- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTank.elkg`: diffs=20 [coordinate=14, section=4, structure=1, ordering=1], first: children[5]/x: number mismatch (251.0 != 513.0)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTank.elkt`: diffs=20 [coordinate=14, section=4, structure=1, ordering=1], first: children[5]/x: number mismatch (251.0 != 513.0)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkg`: diffs=20 [coordinate=16, section=2, ordering=2], first: children[0]/y: number mismatch (96.5 != 358.5)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkt`: diffs=20 [coordinate=16, section=2, ordering=2], first: children[0]/y: number mismatch (96.5 != 358.5)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (653.0 != 610.0)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (653.0 != 610.0)
- `realworld/ptolemy/flattened/aspect_cartrackingattackmodeling_CarTrackingAttackModeling.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1343.0 != 563.0)
- `realworld/ptolemy/flattened/aspect_cartrackingattackmodeling_CarTrackingAttackModeling.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1343.0 != 563.0)
- `realworld/ptolemy/flattened/aspect_cartrackingattackmodeling_CarTrackingAttackModelingAllAttacksInOneModel.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1559.0 != 1629.0)
- `realworld/ptolemy/flattened/aspect_cartrackingattackmodeling_CarTrackingAttackModelingAllAttacksInOneModel.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1559.0 != 1629.0)
- `realworld/ptolemy/flattened/aspect_compositeqm_CheckExecutionTimeConstraints.elkg`: diffs=20 [coordinate=10, section=10], first: children[1]/y: number mismatch (359.5 != 278.5)
- `realworld/ptolemy/flattened/aspect_compositeqm_CheckExecutionTimeConstraints.elkt`: diffs=20 [coordinate=10, section=10], first: children[1]/y: number mismatch (359.5 != 278.5)
- `realworld/ptolemy/flattened/aspect_compositeqm_CompositeQM.elkg`: diffs=19 [section=9, structure=6, coordinate=4], first: children[0]/y: number mismatch (228.00000000000003 != 263.0)
- `realworld/ptolemy/flattened/aspect_compositeqm_CompositeQM.elkt`: diffs=19 [section=9, structure=6, coordinate=4], first: children[0]/y: number mismatch (228.00000000000003 != 263.0)
- `realworld/ptolemy/flattened/aspect_de_DE2.elkg`: diffs=8 [structure=4, coordinate=2, section=2], first: children[19]/y: number mismatch (376.5 != 296.5)
- `realworld/ptolemy/flattened/aspect_de_DE2.elkt`: diffs=8 [structure=4, coordinate=2, section=2], first: children[19]/y: number mismatch (376.5 != 296.5)
- `realworld/ptolemy/flattened/backtrack_primetest_PrimeTest.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1170.0 != 206.0)
- `realworld/ptolemy/flattened/backtrack_primetest_PrimeTest.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1170.0 != 206.0)
- `realworld/ptolemy/flattened/backtrack_ramprollback_RampRollback.elkg`: diffs=20 [coordinate=11, section=8, structure=1], first: children[0]/y: number mismatch (46.0 != 32.0)
- `realworld/ptolemy/flattened/backtrack_ramprollback_RampRollback.elkt`: diffs=20 [coordinate=11, section=8, structure=1], first: children[0]/y: number mismatch (46.0 != 32.0)
