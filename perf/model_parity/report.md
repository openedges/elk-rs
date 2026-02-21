# ELK Model Parity Report

- manifest: `/Users/luuvish/Projects/research/elk-rs/perf/model_parity/rust_manifest.tsv`
- total rows: 1448
- compared rows: 1439
- matched rows: 1164
- drift rows: 275
- skipped rows (java/rust non-ok): 9
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 5374

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 4302 | 80.1% |
| section | 827 | 15.4% |
| structure | 210 | 3.9% |
| ordering | 17 | 0.3% |
| label | 15 | 0.3% |
| other | 3 | 0.1% |

### Top Diff Path Prefixes

- `children[*]/y`: 1451 (27.0%)
- `children[*]/x`: 1135 (21.1%)
- `children[*]/children[*]/y`: 825 (15.4%)
- `children[*]/edges[*]/sections[*]`: 550 (10.2%)
- `children[*]/children[*]/children[*]`: 452 (8.4%)
- `children[*]/children[*]/x`: 322 (6.0%)
- `children[*]/children[*]/edges[*]`: 307 (5.7%)
- `children[*]/edges[*]`: 60 (1.1%)
- `edges[*]/sections[*]/bendPoints[*]`: 54 (1.0%)
- `children[*]/edges[*]/junctionPoints[*]`: 46 (0.9%)

## Drift Samples

- `examples/general/spacing/labels.elkt`: diffs=20 [section=18, coordinate=2], first: children[0]/x: number mismatch (52.0 != 72.0)
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
- `realworld/ptolemy/flattened/ddf_rijndaelencryption_RijndaelEncryption.elkg`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (196.5 != 182.83333333333331)
