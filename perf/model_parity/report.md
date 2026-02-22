# ELK Model Parity Report

- manifest: `/Users/luuvish/Projects/research/elk-rs/perf/model_parity/rust_manifest.tsv`
- total rows: 1448
- compared rows: 1439
- matched rows: 1174
- drift rows: 265
- skipped rows (java/rust non-ok): 9
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 5222

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 4178 | 80.0% |
| section | 817 | 15.6% |
| structure | 195 | 3.7% |
| ordering | 16 | 0.3% |
| label | 13 | 0.2% |
| other | 3 | 0.1% |

### Top Diff Path Prefixes

- `children[*]/y`: 1369 (26.2%)
- `children[*]/x`: 1086 (20.8%)
- `children[*]/children[*]/y`: 828 (15.9%)
- `children[*]/edges[*]/sections[*]`: 555 (10.6%)
- `children[*]/children[*]/children[*]`: 459 (8.8%)
- `children[*]/children[*]/x`: 324 (6.2%)
- `children[*]/children[*]/edges[*]`: 307 (5.9%)
- `children[*]/edges[*]`: 60 (1.1%)
- `edges[*]/sections[*]/bendPoints[*]`: 51 (1.0%)
- `children[*]/edges[*]/junctionPoints[*]`: 46 (0.9%)

## Drift Samples

- `examples/general/spacing/labels.elkt`: diffs=20 [section=18, coordinate=2], first: children[0]/x: number mismatch (52.0 != 72.0)
- `examples/hierarchy/hierarchicalLayoutMixing.elkt`: diffs=20 [coordinate=11, section=9], first: children[0]/children[0]/y: number mismatch (12.0 != 76.0)
- `examples/user-hints/model-order/modelOrderCycleBreaking.elkt`: diffs=20 [section=11, coordinate=8, structure=1], first: children[0]/children[0]/x: number mismatch (62.0 != 82.0)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkg`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (653.0 != 610.0)
- `realworld/ptolemy/flattened/algebraic_rlc_RLC.elkt`: diffs=20 [coordinate=20], first: children[0]/x: number mismatch (653.0 != 610.0)
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
- `realworld/ptolemy/flattened/ddf_rijndaelencryption_RijndaelEncryption.elkt`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (196.5 != 182.83333333333331)
- `realworld/ptolemy/flattened/de_clock_ClockTest.elkg`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (394.9 != 292.6333333333333)
