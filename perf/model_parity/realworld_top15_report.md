# ELK Model Parity Report

- manifest: `perf/model_parity/rust_manifest_realworld_top15.tsv`
- total rows: 15
- compared rows: 15
- matched rows: 0
- drift rows: 15
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 200
- total diffs across all models: 2136

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| section | 1331 | 62.3% |
| coordinate | 600 | 28.1% |
| structure | 161 | 7.5% |
| ordering | 32 | 1.5% |
| other | 12 | 0.6% |

### Top Diff Path Prefixes

- `edges[*]/sections[*]/endPoint`: 449 (21.0%)
- `edges[*]/sections[*]/startPoint`: 446 (20.9%)
- `edges[*]/sections[*]/bendPoints[*]`: 436 (20.4%)
- `children[*]/y`: 337 (15.8%)
- `children[*]/x`: 263 (12.3%)
- `edges[*]`: 81 (3.8%)
- `edges[*]/sections[*]`: 80 (3.7%)
- `edges[*]/sections[*]/bendPoints`: 32 (1.5%)
- `height`: 9 (0.4%)
- `width`: 3 (0.1%)

## Drift Samples

- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTank.elkg`: diffs=69 [section=45, coordinate=16, structure=6, ordering=1, other=1], first: children[0]/y: number mismatch (32.0 != 42.0)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTank.elkt`: diffs=69 [section=45, coordinate=16, structure=6, ordering=1, other=1], first: children[0]/y: number mismatch (32.0 != 42.0)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkg`: diffs=187 [section=143, coordinate=22, structure=15, ordering=6, other=1], first: children[0]/y: number mismatch (96.5 != 120.0)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkt`: diffs=187 [section=143, coordinate=22, structure=15, ordering=6, other=1], first: children[0]/y: number mismatch (96.5 != 120.0)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkg`: diffs=107 [section=68, coordinate=24, structure=11, ordering=2, other=2], first: children[0]/x: number mismatch (653.0 != 657.0)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkt`: diffs=107 [section=68, coordinate=24, structure=11, ordering=2, other=2], first: children[0]/x: number mismatch (653.0 != 657.0)
- `realworld/ptolemy/flattened/aspect_cartrackingattackmodeling_CarTrackingAttackModeling.elkg`: diffs=200 [section=113, coordinate=72, structure=13, ordering=2], first: children[0]/x: number mismatch (1343.0 != 563.0)
- `realworld/ptolemy/flattened/aspect_cartrackingattackmodeling_CarTrackingAttackModeling.elkt`: diffs=200 [section=113, coordinate=72, structure=13, ordering=2], first: children[0]/x: number mismatch (1343.0 != 563.0)
- `realworld/ptolemy/flattened/aspect_cartrackingattackmodeling_CarTrackingAttackModelingAllAttacksInOneModel.elkg`: diffs=200 [coordinate=101, section=84, structure=12, ordering=3], first: children[0]/x: number mismatch (1559.0 != 1659.0)
- `realworld/ptolemy/flattened/aspect_cartrackingattackmodeling_CarTrackingAttackModelingAllAttacksInOneModel.elkt`: diffs=200 [coordinate=101, section=84, structure=12, ordering=3], first: children[0]/x: number mismatch (1559.0 != 1659.0)
- `realworld/ptolemy/flattened/aspect_compositeqm_CheckExecutionTimeConstraints.elkg`: diffs=50 [section=34, coordinate=11, structure=4, other=1], first: children[1]/y: number mismatch (359.5 != 276.0)
- `realworld/ptolemy/flattened/aspect_compositeqm_CheckExecutionTimeConstraints.elkt`: diffs=50 [section=34, coordinate=11, structure=4, other=1], first: children[1]/y: number mismatch (359.5 != 276.0)
- `realworld/ptolemy/flattened/backtrack_primetest_PrimeTest.elkg`: diffs=200 [section=139, coordinate=44, structure=16, ordering=1], first: children[0]/x: number mismatch (1170.0 != 1119.0)
- `realworld/ptolemy/flattened/backtrack_primetest_PrimeTest.elkt`: diffs=200 [section=139, coordinate=44, structure=16, ordering=1], first: children[0]/x: number mismatch (1170.0 != 1119.0)
- `realworld/ptolemy/flattened/backtrack_ramprollback_RampRollback.elkg`: diffs=110 [section=79, coordinate=20, structure=7, ordering=2, other=2], first: children[0]/y: number mismatch (46.0 != 108.5)
