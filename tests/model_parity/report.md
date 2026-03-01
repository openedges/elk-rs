# ELK Model Parity Report

- manifest: `/Users/luuvish/Projects/research/elk-rs/tests/model_parity/rust_manifest.tsv`
- total rows: 1448
- compared rows: 1439
- matched rows: 1350
- drift rows: 89
- skipped rows (java/rust non-ok): 9
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 1565

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| section | 945 | 60.4% |
| coordinate | 617 | 39.4% |
| other | 2 | 0.1% |
| label | 1 | 0.1% |

### Top Diff Path Prefixes

- `children[*]/y`: 484 (30.9%)
- `edges[*]/sections[*]/bendPoints[*]`: 315 (20.1%)
- `children[*]/edges[*]/sections[*]`: 242 (15.5%)
- `children[*]/children[*]/edges[*]`: 140 (8.9%)
- `edges[*]/sections[*]/endPoint`: 130 (8.3%)
- `edges[*]/sections[*]/startPoint`: 128 (8.2%)
- `children[*]/children[*]/y`: 51 (3.3%)
- `children[*]/children[*]/children[*]`: 36 (2.3%)
- `edges[*]/junctionPoints[*]/y`: 22 (1.4%)
- `children[*]/x`: 7 (0.4%)

## Drift Samples

- `examples/edges/insideSelfLoops.elkt`: diffs=18 [section=16, coordinate=2], first: children[0]/y: number mismatch (22.0 != 12.0)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModule.elkg`: diffs=20 [section=15, coordinate=5], first: children[0]/y: number mismatch (437.6666666666667 != 427.6666666666667)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModule.elkt`: diffs=20 [section=15, coordinate=5], first: children[0]/y: number mismatch (437.6666666666667 != 427.6666666666667)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModuleNonBacktrack.elkg`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (51.5 != 42.0)
- `realworld/ptolemy/flattened/backtrack_trialmodule_TrialModuleNonBacktrack.elkt`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (51.5 != 42.0)
- `realworld/ptolemy/flattened/de_printingpress_PrintingPress-Validation.elkg`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (989.8333333333335 != 982.3333333333335)
- `realworld/ptolemy/flattened/de_printingpress_PrintingPress-Validation.elkt`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (989.8333333333335 != 982.3333333333335)
- `realworld/ptolemy/flattened/ptera_adaptivecarwash_AdaptiveCarWash.elkg`: diffs=20 [coordinate=11, section=9], first: children[1]/y: number mismatch (182.5 != 172.5)
- `realworld/ptolemy/flattened/ptera_adaptivecarwash_AdaptiveCarWash.elkt`: diffs=20 [coordinate=11, section=9], first: children[1]/y: number mismatch (182.5 != 172.5)
- `realworld/ptolemy/flattened/ptera_adaptivecarwash_AdaptiveCarWashFSM.elkg`: diffs=2 [section=2], first: edges[11]/sections[0]/bendPoints[1]/y: number mismatch (152.5 != 232.5)
- `realworld/ptolemy/flattened/ptera_adaptivecarwash_AdaptiveCarWashFSM.elkt`: diffs=2 [section=2], first: edges[11]/sections[0]/bendPoints[1]/y: number mismatch (152.5 != 232.5)
- `realworld/ptolemy/flattened/ptera_simultaneouscarwash_SimultaneousCarWash.elkg`: diffs=12 [section=10, coordinate=2], first: children[3]/y: number mismatch (392.0 != 382.0)
- `realworld/ptolemy/flattened/ptera_simultaneouscarwash_SimultaneousCarWash.elkt`: diffs=12 [section=10, coordinate=2], first: children[3]/y: number mismatch (392.0 != 382.0)
- `realworld/ptolemy/flattened/ptera_trafficlight_TrafficLight.elkg`: diffs=10 [section=8, coordinate=2], first: children[5]/y: number mismatch (42.0 != 32.0)
- `realworld/ptolemy/flattened/ptera_trafficlight_TrafficLight.elkt`: diffs=10 [section=8, coordinate=2], first: children[5]/y: number mismatch (42.0 != 32.0)
- `realworld/ptolemy/flattened/ptides_distributedpowerplant_DistributedPowerPlant.elkg`: diffs=20 [section=19, coordinate=1], first: children[0]/x: number mismatch (249.0 != 239.0)
- `realworld/ptolemy/flattened/ptides_distributedpowerplant_DistributedPowerPlant.elkt`: diffs=20 [section=19, coordinate=1], first: children[0]/x: number mismatch (249.0 != 239.0)
- `realworld/ptolemy/flattened/ptides_multiplatformtdma_MultiPlatformTDMA.elkg`: diffs=20 [section=17, coordinate=3], first: children[37]/y: number mismatch (42.0 != 32.0)
- `realworld/ptolemy/flattened/ptides_multiplatformtdma_MultiPlatformTDMA.elkt`: diffs=20 [section=17, coordinate=3], first: children[37]/y: number mismatch (42.0 != 32.0)
- `realworld/ptolemy/flattened/ptides_powerplant_PowerPlant.elkg`: diffs=20 [section=19, coordinate=1], first: children[0]/x: number mismatch (249.0 != 239.0)
