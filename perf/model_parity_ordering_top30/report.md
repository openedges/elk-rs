# ELK Model Parity Report

- manifest: `perf/model_parity_ordering_top30/rust_manifest.tsv`
- total rows: 20
- compared rows: 20
- matched rows: 0
- drift rows: 20
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 400

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 352 | 88.0% |
| section | 40 | 10.0% |
| ordering | 4 | 1.0% |
| structure | 4 | 1.0% |

### Top Diff Path Prefixes

- `children[*]/y`: 258 (64.5%)
- `children[*]/x`: 92 (23.0%)
- `edges[*]/sections[*]/endPoint`: 16 (4.0%)
- `edges[*]/sections[*]/startPoint`: 14 (3.5%)
- `edges[*]/sections[*]/bendPoints[*]`: 10 (2.5%)
- `edges[*]/sections[*]`: 4 (1.0%)
- `edges[*]/junctionPoints`: 2 (0.5%)
- `edges[*]/sections[*]/bendPoints`: 2 (0.5%)
- `edges[*]/junctionPoints[*]/y`: 2 (0.5%)

## Drift Samples

- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkg`: diffs=20 [coordinate=16, section=2, ordering=2], first: children[0]/y: number mismatch (96.5 != 358.5)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkt`: diffs=20 [coordinate=16, section=2, ordering=2], first: children[0]/y: number mismatch (96.5 != 358.5)
- `realworld/ptolemy/flattened/aspect_compositeqm_CheckExecutionTimeConstraints.elkg`: diffs=20 [coordinate=10, section=10], first: children[1]/y: number mismatch (359.5 != 278.5)
- `realworld/ptolemy/flattened/aspect_compositeqm_CheckExecutionTimeConstraints.elkt`: diffs=20 [coordinate=10, section=10], first: children[1]/y: number mismatch (359.5 != 278.5)
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
