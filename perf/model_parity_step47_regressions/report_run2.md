# ELK Model Parity Report

- manifest: `perf/model_parity_step47_regressions/rust_manifest_run2.tsv`
- total rows: 41
- compared rows: 41
- matched rows: 3
- drift rows: 38
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 686

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 396 | 57.7% |
| section | 290 | 42.3% |

### Top Diff Path Prefixes

- `children[*]/y`: 368 (53.6%)
- `edges[*]/sections[*]/bendPoints[*]`: 136 (19.8%)
- `edges[*]/sections[*]/startPoint`: 64 (9.3%)
- `edges[*]/sections[*]/endPoint`: 62 (9.0%)
- `children[*]/edges[*]/sections[*]`: 28 (4.1%)
- `edges[*]/junctionPoints[*]/y`: 18 (2.6%)
- `children[*]/children[*]/y`: 10 (1.5%)

## Drift Samples

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
- `realworld/ptolemy/flattened/ptides_multiplatformtdma_MultiPlatformTDMA.elkg`: diffs=20 [section=17, coordinate=3], first: children[37]/y: number mismatch (42.0 != 32.0)
- `realworld/ptolemy/flattened/ptides_multiplatformtdma_MultiPlatformTDMA.elkt`: diffs=20 [section=17, coordinate=3], first: children[37]/y: number mismatch (42.0 != 32.0)
- `realworld/ptolemy/flattened/ptides_printingpress_PrintingPress.elkg`: diffs=20 [coordinate=20], first: children[1]/y: number mismatch (1950.6666666666665 != 1943.1666666666665)
- `realworld/ptolemy/flattened/ptides_printingpress_PrintingPress.elkt`: diffs=20 [coordinate=20], first: children[1]/y: number mismatch (1950.6666666666665 != 1943.1666666666665)
- `realworld/ptolemy/flattened/ptides_trex_TREX.elkg`: diffs=20 [coordinate=12, section=8], first: children[2]/y: number mismatch (42.0 != 32.0)
- `realworld/ptolemy/flattened/ptides_trex_TREX.elkt`: diffs=20 [coordinate=12, section=8], first: children[2]/y: number mismatch (42.0 != 32.0)
