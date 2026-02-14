# ELK Model Parity Report

- manifest: `perf/model_parity_ordering_top30/rust_manifest.tsv`
- total rows: 20
- compared rows: 20
- matched rows: 2
- drift rows: 18
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 360

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 332 | 92.2% |
| section | 20 | 5.6% |
| ordering | 4 | 1.1% |
| structure | 4 | 1.1% |

### Top Diff Path Prefixes

- `children[*]/y`: 240 (66.7%)
- `children[*]/x`: 92 (25.6%)
- `edges[*]/sections[*]/endPoint`: 8 (2.2%)
- `edges[*]/sections[*]/startPoint`: 8 (2.2%)
- `edges[*]/sections[*]/bendPoints[*]`: 4 (1.1%)
- `edges[*]/sections[*]`: 4 (1.1%)
- `edges[*]/junctionPoints`: 2 (0.6%)
- `edges[*]/sections[*]/bendPoints`: 2 (0.6%)

## Drift Samples

- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkg`: diffs=20 [coordinate=16, section=2, ordering=2], first: children[0]/y: number mismatch (96.5 != 358.5)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkt`: diffs=20 [coordinate=16, section=2, ordering=2], first: children[0]/y: number mismatch (96.5 != 358.5)
- `realworld/ptolemy/flattened/backtrack_ramprollback_RampRollback.elkg`: diffs=20 [coordinate=11, section=8, structure=1], first: children[0]/y: number mismatch (46.0 != 32.0)
- `realworld/ptolemy/flattened/backtrack_ramprollback_RampRollback.elkt`: diffs=20 [coordinate=11, section=8, structure=1], first: children[0]/y: number mismatch (46.0 != 32.0)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModule.elkg`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (427.6666666666667 != 96.0)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModule.elkt`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (427.6666666666667 != 96.0)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModuleNonBacktrack.elkg`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (42.0 != 32.5)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModuleNonBacktrack.elkt`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (42.0 != 32.5)
- `realworld/ptolemy/flattened/ca_conway_Conway.elkg`: diffs=20 [coordinate=20], first: children[1]/x: number mismatch (961.0 != 991.0)
- `realworld/ptolemy/flattened/ca_conway_Conway.elkt`: diffs=20 [coordinate=20], first: children[1]/x: number mismatch (961.0 != 991.0)
- `realworld/ptolemy/flattened/ci_router_dropqueuetest1.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (477.0 != 497.0)
- `realworld/ptolemy/flattened/ci_router_dropqueuetest1.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (477.0 != 497.0)
- `realworld/ptolemy/flattened/ci_router_queuetest1.elkg`: diffs=20 [coordinate=19, structure=1], first: children[0]/x: number mismatch (245.0 != 255.0)
- `realworld/ptolemy/flattened/ci_router_queuetest1.elkt`: diffs=20 [coordinate=19, structure=1], first: children[0]/x: number mismatch (245.0 != 255.0)
- `realworld/ptolemy/flattened/colt_coltrandom_ColtRandom.elkg`: diffs=20 [coordinate=20], first: children[1]/y: number mismatch (275.5 != 113.5)
- `realworld/ptolemy/flattened/colt_coltrandom_ColtRandom.elkt`: diffs=20 [coordinate=20], first: children[1]/y: number mismatch (275.5 != 113.5)
- `realworld/ptolemy/flattened/comm_trellisdecoder_TrellisDecoder.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (389.0 != 399.0)
- `realworld/ptolemy/flattened/comm_trellisdecoder_TrellisDecoder.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (389.0 != 399.0)
