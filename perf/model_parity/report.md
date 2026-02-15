# ELK Model Parity Report

- manifest: `/Users/luuvish/Projects/research/elk-rs/perf/model_parity/rust_manifest.tsv`
- total rows: 1448
- compared rows: 1439
- matched rows: 731
- drift rows: 708
- skipped rows (java/rust non-ok): 9
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 13524

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 8797 | 65.0% |
| section | 4341 | 32.1% |
| structure | 165 | 1.2% |
| label | 100 | 0.7% |
| other | 65 | 0.5% |
| ordering | 56 | 0.4% |

### Top Diff Path Prefixes

- `children[*]/edges[*]/sections[*]`: 2535 (18.7%)
- `children[*]/y`: 2212 (16.4%)
- `children[*]/children[*]/x`: 1744 (12.9%)
- `children[*]/x`: 1384 (10.2%)
- `children[*]/children[*]/children[*]`: 1101 (8.1%)
- `children[*]/children[*]/y`: 1074 (7.9%)
- `children[*]/children[*]/edges[*]`: 991 (7.3%)
- `children[*]/ports[*]/y`: 366 (2.7%)
- `edges[*]/sections[*]/bendPoints[*]`: 336 (2.5%)
- `children[*]/ports[*]/x`: 278 (2.1%)

## Drift Samples

- `examples/edges/insideSelfLoops.elkt`: diffs=20 [section=16, coordinate=4], first: children[0]/ports[1]/x: number mismatch (4.0 != 24.0)
- `examples/general/spacing/labels.elkt`: diffs=20 [section=18, coordinate=2], first: children[0]/x: number mismatch (52.0 != 72.0)
- `examples/general/spacing/ports.elkt`: diffs=7 [coordinate=7], first: children[0]/ports[0]/x: number mismatch (16.666666666666668 != 33.333333333333336)
- `examples/general/spacing/portsSurrounding.elkt`: diffs=10 [coordinate=10], first: children[0]/ports[0]/y: number mismatch (57.0 != 57.5)
- `examples/hierarchy/hierarchicalLayoutMixing.elkt`: diffs=20 [coordinate=9, section=8, structure=3], first: children[0]/children[0]/y: number mismatch (12.0 != 76.0)
- `examples/labels/portLabelsMulti.elkt`: diffs=4 [label=4], first: children[1]/ports[2]/labels[0]/y: number mismatch (1.0 != -31.0)
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
