# ELK Model Parity Report

- manifest: `/Users/luuvish/Projects/research/elk-rs/perf/model_parity/rust_manifest.tsv`
- total rows: 1448
- compared rows: 1437
- matched rows: 1151
- drift rows: 286
- skipped rows (java/rust non-ok): 11
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 5612

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 4382 | 78.1% |
| section | 972 | 17.3% |
| structure | 214 | 3.8% |
| label | 24 | 0.4% |
| ordering | 17 | 0.3% |
| other | 3 | 0.1% |

### Top Diff Path Prefixes

- `children[*]/y`: 1469 (26.2%)
- `children[*]/x`: 1136 (20.2%)
- `children[*]/children[*]/y`: 876 (15.6%)
- `children[*]/edges[*]/sections[*]`: 649 (11.6%)
- `children[*]/children[*]/children[*]`: 438 (7.8%)
- `children[*]/children[*]/x`: 332 (5.9%)
- `children[*]/children[*]/edges[*]`: 277 (4.9%)
- `edges[*]/sections[*]/bendPoints[*]`: 98 (1.7%)
- `children[*]/edges[*]`: 68 (1.2%)
- `edges[*]/sections[*]/endPoint`: 54 (1.0%)

## Drift Samples

- `examples/general/spacing/labels.elkt`: diffs=20 [section=18, coordinate=2], first: children[0]/x: number mismatch (52.0 != 72.0)
- `examples/general/spacing/portsSurrounding.elkt`: diffs=6 [coordinate=6], first: children[0]/ports[0]/y: number mismatch (57.0 != 57.5)
- `examples/hierarchy/hierarchicalLayoutMixing.elkt`: diffs=20 [coordinate=11, section=9], first: children[0]/children[0]/y: number mismatch (12.0 != 76.0)
- `examples/user-hints/model-order/modelOrderCycleBreaking.elkt`: diffs=20 [section=11, coordinate=8, structure=1], first: children[0]/children[0]/x: number mismatch (62.0 != 82.0)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (653.0 != 610.0)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (653.0 != 610.0)
- `realworld/ptolemy/flattened/aspect_de_DE2.elkg`: diffs=8 [structure=4, coordinate=2, section=2], first: children[19]/y: number mismatch (376.5 != 296.5)
- `realworld/ptolemy/flattened/aspect_de_DE2.elkt`: diffs=8 [structure=4, coordinate=2, section=2], first: children[19]/y: number mismatch (376.5 != 296.5)
- `realworld/ptolemy/flattened/backtrack_primetest_PrimeTest.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1170.0 != 206.0)
- `realworld/ptolemy/flattened/backtrack_primetest_PrimeTest.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1170.0 != 206.0)
- `realworld/ptolemy/flattened/continuous_cartracking_CarTracking.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1762.0 != 1754.0)
- `realworld/ptolemy/flattened/continuous_cartracking_CarTracking.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (1762.0 != 1754.0)
- `realworld/ptolemy/flattened/continuous_hierarchicalexecution_HierarchicalExecution.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (125.0 != 293.0)
- `realworld/ptolemy/flattened/continuous_hierarchicalexecution_HierarchicalExecution.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (125.0 != 293.0)
- `realworld/ptolemy/flattened/continuous_sigmadelta_SigmaDelta.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (334.0 != 244.0)
- `realworld/ptolemy/flattened/continuous_sigmadelta_SigmaDelta.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (334.0 != 244.0)
- `realworld/ptolemy/flattened/continuous_starmac_Starmac.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (588.0 != 578.0)
- `realworld/ptolemy/flattened/continuous_starmac_Starmac.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (588.0 != 578.0)
- `realworld/ptolemy/flattened/continuous_staticunits_StaticUnits.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (113.0 != 133.0)
- `realworld/ptolemy/flattened/continuous_staticunits_StaticUnits.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (113.0 != 133.0)
