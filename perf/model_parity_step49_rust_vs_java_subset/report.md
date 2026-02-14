# ELK Model Parity Report

- manifest: `perf/model_parity_step49_rust_vs_java_subset/rust_manifest.tsv`
- total rows: 38
- compared rows: 38
- matched rows: 20
- drift rows: 18
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 360

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 250 | 69.4% |
| section | 110 | 30.6% |

### Top Diff Path Prefixes

- `children[*]/y`: 238 (66.1%)
- `edges[*]/sections[*]/bendPoints[*]`: 54 (15.0%)
- `edges[*]/sections[*]/endPoint`: 28 (7.8%)
- `edges[*]/sections[*]/startPoint`: 28 (7.8%)
- `edges[*]/junctionPoints[*]/y`: 12 (3.3%)

## Drift Samples

- `realworld/ptolemy/flattened/de_printingpress_PrintingPress-Validation.elkg`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (989.8333333333335 != 982.3333333333335)
- `realworld/ptolemy/flattened/de_printingpress_PrintingPress-Validation.elkt`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (989.8333333333335 != 982.3333333333335)
- `realworld/ptolemy/flattened/ptides_printingpress_PrintingPress.elkg`: diffs=20 [coordinate=20], first: children[1]/y: number mismatch (1950.6666666666665 != 1943.1666666666665)
- `realworld/ptolemy/flattened/ptides_printingpress_PrintingPress.elkt`: diffs=20 [coordinate=20], first: children[1]/y: number mismatch (1950.6666666666665 != 1943.1666666666665)
- `realworld/ptolemy/flattened/ptides_trex_TREX.elkg`: diffs=20 [coordinate=12, section=8], first: children[2]/y: number mismatch (42.0 != 32.0)
- `realworld/ptolemy/flattened/ptides_trex_TREX.elkt`: diffs=20 [coordinate=12, section=8], first: children[2]/y: number mismatch (42.0 != 32.0)
- `realworld/ptolemy/flattened/ptides_trex_TREXNetworkV1.elkg`: diffs=20 [coordinate=15, section=5], first: children[18]/y: number mismatch (133.0 != 123.0)
- `realworld/ptolemy/flattened/ptides_trex_TREXNetworkV1.elkt`: diffs=20 [coordinate=15, section=5], first: children[18]/y: number mismatch (133.0 != 123.0)
- `realworld/ptolemy/flattened/ptides_trex_TREXNoNetworkAllDigital.elkg`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (1017.3333333333335 != 1027.3333333333335)
- `realworld/ptolemy/flattened/ptides_trex_TREXNoNetworkAllDigital.elkt`: diffs=20 [coordinate=20], first: children[0]/y: number mismatch (1017.3333333333335 != 1027.3333333333335)
- `realworld/ptolemy/flattened/ptolemy_probabilisticmodels_ChannelFaultModel.elkg`: diffs=20 [section=12, coordinate=8], first: children[0]/y: number mismatch (396.0 != 386.0)
- `realworld/ptolemy/flattened/ptolemy_probabilisticmodels_ChannelFaultModel.elkt`: diffs=20 [section=12, coordinate=8], first: children[0]/y: number mismatch (396.0 != 386.0)
- `realworld/ptolemy/flattened/ptolemy_test_auto_ChannelFaultModel.elkg`: diffs=20 [coordinate=11, section=9], first: children[0]/y: number mismatch (396.0 != 386.0)
- `realworld/ptolemy/flattened/ptolemy_test_auto_ChannelFaultModel.elkt`: diffs=20 [coordinate=11, section=9], first: children[0]/y: number mismatch (396.0 != 386.0)
- `realworld/ptolemy/hierarchical/ptolemy_probabilisticmodels_ChannelFaultModel.elkg`: diffs=20 [section=12, coordinate=8], first: children[0]/y: number mismatch (521.0 != 511.0)
- `realworld/ptolemy/hierarchical/ptolemy_probabilisticmodels_ChannelFaultModel.elkt`: diffs=20 [section=12, coordinate=8], first: children[0]/y: number mismatch (521.0 != 511.0)
- `realworld/ptolemy/hierarchical/ptolemy_test_auto_ChannelFaultModel.elkg`: diffs=20 [coordinate=11, section=9], first: children[0]/y: number mismatch (521.0 != 511.0)
- `realworld/ptolemy/hierarchical/ptolemy_test_auto_ChannelFaultModel.elkt`: diffs=20 [coordinate=11, section=9], first: children[0]/y: number mismatch (521.0 != 511.0)
