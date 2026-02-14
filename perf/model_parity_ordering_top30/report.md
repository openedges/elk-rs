# ELK Model Parity Report

- manifest: `perf/model_parity_ordering_top30/rust_manifest.tsv`
- total rows: 20
- compared rows: 20
- matched rows: 12
- drift rows: 8
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 116

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| section | 58 | 50.0% |
| coordinate | 38 | 32.8% |
| ordering | 10 | 8.6% |
| structure | 8 | 6.9% |
| other | 2 | 1.7% |

### Top Diff Path Prefixes

- `children[*]/y`: 30 (25.9%)
- `edges[*]/sections[*]/bendPoints[*]`: 24 (20.7%)
- `edges[*]/sections[*]/endPoint`: 20 (17.2%)
- `edges[*]/sections[*]/startPoint`: 14 (12.1%)
- `children[*]/x`: 6 (5.2%)
- `edges[*]/junctionPoints`: 6 (5.2%)
- `edges[*]`: 6 (5.2%)
- `edges[*]/sections[*]/bendPoints`: 4 (3.4%)
- `edges[*]/junctionPoints[*]/x`: 2 (1.7%)
- `edges[*]/sections[*]`: 2 (1.7%)

## Drift Samples

- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkg`: diffs=20 [section=13, coordinate=5, ordering=2], first: children[4]/x: number mismatch (801.0 != 791.0)
- `realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkt`: diffs=20 [section=13, coordinate=5, ordering=2], first: children[4]/x: number mismatch (801.0 != 791.0)
- `realworld/ptolemy/flattened/backtrack_ramprollback_RampRollback.elkg`: diffs=20 [coordinate=11, section=8, structure=1], first: children[0]/y: number mismatch (46.0 != 42.0)
- `realworld/ptolemy/flattened/backtrack_ramprollback_RampRollback.elkt`: diffs=20 [coordinate=11, section=8, structure=1], first: children[0]/y: number mismatch (46.0 != 42.0)
- `realworld/ptolemy/flattened/ca_conway_Conway.elkg`: diffs=3 [structure=2, ordering=1], first: edges[26]: missing keys on right: junctionPoints
- `realworld/ptolemy/flattened/ca_conway_Conway.elkt`: diffs=3 [structure=2, ordering=1], first: edges[26]: missing keys on right: junctionPoints
- `realworld/ptolemy/flattened/comm_trellisdecoder_TrellisDecoder.elkg`: diffs=15 [section=8, coordinate=3, ordering=2, structure=1, other=1], first: children[19]/y: number mismatch (381.50000000000006 != 439.6666666666667)
- `realworld/ptolemy/flattened/comm_trellisdecoder_TrellisDecoder.elkt`: diffs=15 [section=8, coordinate=3, ordering=2, structure=1, other=1], first: children[19]/y: number mismatch (381.50000000000006 != 439.6666666666667)
