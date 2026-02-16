# ELK Model Parity Report

- manifest: `/Users/luuvish/Projects/research/elk-rs/perf/model_parity/rust_manifest.tsv`
- total rows: 1448
- compared rows: 1439
- matched rows: 754
- drift rows: 685
- skipped rows (java/rust non-ok): 9
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 13249

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 8680 | 65.5% |
| section | 4224 | 31.9% |
| structure | 169 | 1.3% |
| label | 89 | 0.7% |
| other | 51 | 0.4% |
| ordering | 36 | 0.3% |

### Top Diff Path Prefixes

- `children[*]/edges[*]/sections[*]`: 2521 (19.0%)
- `children[*]/y`: 2168 (16.4%)
- `children[*]/children[*]/x`: 1736 (13.1%)
- `children[*]/x`: 1312 (9.9%)
- `children[*]/children[*]/children[*]`: 1095 (8.3%)
- `children[*]/children[*]/y`: 1050 (7.9%)
- `children[*]/children[*]/edges[*]`: 982 (7.4%)
- `children[*]/ports[*]/y`: 353 (2.7%)
- `edges[*]/sections[*]/bendPoints[*]`: 331 (2.5%)
- `children[*]/ports[*]/x`: 324 (2.4%)

## Drift Samples

- `examples/edges/insideSelfLoops.elkt`: diffs=20 [section=13, coordinate=6, structure=1], first: children[0]/ports[1]/x: number mismatch (4.0 != 24.0)
- `examples/general/spacing/labels.elkt`: diffs=20 [section=18, coordinate=2], first: children[0]/x: number mismatch (52.0 != 72.0)
- `examples/general/spacing/portsSurrounding.elkt`: diffs=6 [coordinate=6], first: children[0]/ports[0]/y: number mismatch (57.0 != 57.5)
- `examples/hierarchy/hierarchicalLayoutMixing.elkt`: diffs=20 [coordinate=9, section=8, structure=3], first: children[0]/children[0]/y: number mismatch (12.0 != 76.0)
- `examples/ports/portConstraints.elkt`: diffs=13 [coordinate=7, section=5, other=1], first: children[0]/children[0]/ports[0]/x: number mismatch (0.0 != -5.0)
- `examples/user-hints/layered/verticalOrder.elkt`: diffs=9 [coordinate=4, structure=2, section=2, label=1], first: children[1]/children[1]/y: number mismatch (52.0 != 32.0)
- `examples/user-hints/model-order/modelOrderCycleBreaking.elkt`: diffs=20 [section=11, coordinate=8, structure=1], first: children[0]/children[0]/x: number mismatch (62.0 != 82.0)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (653.0 != 610.0)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (653.0 != 610.0)
- `realworld/ptolemy/flattened/backtrack_primetest_PrimeTest.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1170.0 != 206.0)
- `realworld/ptolemy/flattened/backtrack_primetest_PrimeTest.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1170.0 != 206.0)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModule.elkg`: diffs=20 [section=15, coordinate=5], first: children[0]/y: number mismatch (437.6666666666667 != 427.6666666666667)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModule.elkt`: diffs=20 [section=15, coordinate=5], first: children[0]/y: number mismatch (437.6666666666667 != 427.6666666666667)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModuleNonBacktrack.elkg`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (51.5 != 42.0)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModuleNonBacktrack.elkt`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (51.5 != 42.0)
- `realworld/ptolemy/flattened/continuous_cartracking_CarTracking.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1762.0 != 1754.0)
- `realworld/ptolemy/flattened/continuous_cartracking_CarTracking.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1762.0 != 1754.0)
- `realworld/ptolemy/flattened/continuous_hierarchicalexecution_HierarchicalExecution.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (125.0 != 293.0)
- `realworld/ptolemy/flattened/continuous_hierarchicalexecution_HierarchicalExecution.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (125.0 != 293.0)
- `realworld/ptolemy/flattened/continuous_sigmadelta_SigmaDelta.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (334.0 != 244.0)
