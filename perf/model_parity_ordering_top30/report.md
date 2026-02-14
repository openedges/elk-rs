# ELK Model Parity Report

- manifest: `perf/model_parity_ordering_top30/rust_manifest.tsv`
- total rows: 20
- compared rows: 20
- matched rows: 6
- drift rows: 14
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 278

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 226 | 81.3% |
| section | 34 | 12.2% |
| structure | 12 | 4.3% |
| ordering | 6 | 2.2% |

### Top Diff Path Prefixes

- `children[*]/y`: 170 (61.2%)
- `children[*]/x`: 52 (18.7%)
- `edges[*]/sections[*]/startPoint`: 16 (5.8%)
- `edges[*]/sections[*]/endPoint`: 10 (3.6%)
- `edges[*]/sections[*]/bendPoints[*]`: 8 (2.9%)
- `edges[*]`: 8 (2.9%)
- `edges[*]/sections[*]/bendPoints`: 4 (1.4%)
- `edges[*]/sections[*]`: 4 (1.4%)
- `edges[*]/junctionPoints[*]/y`: 4 (1.4%)
- `edges[*]/junctionPoints`: 2 (0.7%)

## Drift Samples

- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkg`: diffs=20 [coordinate=16, section=2, ordering=2], first: children[0]/y: number mismatch (96.5 != 345.5)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkt`: diffs=20 [coordinate=16, section=2, ordering=2], first: children[0]/y: number mismatch (96.5 != 345.5)
- `realworld/ptolemy/flattened/backtrack_ramprollback_RampRollback.elkg`: diffs=20 [coordinate=11, section=9], first: children[0]/y: number mismatch (46.0 != 35.0)
- `realworld/ptolemy/flattened/backtrack_ramprollback_RampRollback.elkt`: diffs=20 [coordinate=11, section=9], first: children[0]/y: number mismatch (46.0 != 35.0)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModule.elkg`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (427.6666666666667 != 42.0)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModule.elkt`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (427.6666666666667 != 42.0)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModuleNonBacktrack.elkg`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (42.0 != 32.5)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModuleNonBacktrack.elkt`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (42.0 != 32.5)
- `realworld/ptolemy/flattened/ca_conway_Conway.elkg`: diffs=19 [coordinate=6, structure=6, section=6, ordering=1], first: children[11]/y: number mismatch (32.0 != 227.0)
- `realworld/ptolemy/flattened/ca_conway_Conway.elkt`: diffs=19 [coordinate=6, structure=6, section=6, ordering=1], first: children[11]/y: number mismatch (32.0 != 227.0)
- `realworld/ptolemy/flattened/colt_coltrandom_ColtRandom.elkg`: diffs=20 [coordinate=20], first: children[1]/y: number mismatch (275.5 != 113.5)
- `realworld/ptolemy/flattened/colt_coltrandom_ColtRandom.elkt`: diffs=20 [coordinate=20], first: children[1]/y: number mismatch (275.5 != 113.5)
- `realworld/ptolemy/flattened/comm_trellisdecoder_TrellisDecoder.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (389.0 != 399.0)
- `realworld/ptolemy/flattened/comm_trellisdecoder_TrellisDecoder.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (389.0 != 399.0)
