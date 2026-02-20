# ELK Model Parity Report

- manifest: `/Users/cody.ij.hwang/Projects/github/elk-rs/perf/model_parity_tickets/rust_manifest.tsv`
- total rows: 110
- compared rows: 109
- matched rows: 107
- drift rows: 2
- skipped rows (java/rust non-ok): 1
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 40

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 36 | 90.0% |
| label | 4 | 10.0% |

### Top Diff Path Prefixes

- `children[*]/children[*]/children[*]`: 15 (37.5%)
- `children[*]/children[*]/y`: 12 (30.0%)
- `children[*]/children[*]/x`: 10 (25.0%)
- `children[*]/children[*]/height`: 1 (2.5%)
- `children[*]/children[*]/labels[*]`: 1 (2.5%)
- `children[*]/children[*]/width`: 1 (2.5%)

## Drift Samples

- `tickets/layered/213_componentsCompaction.elkt`: diffs=20 [coordinate=20], first: children[0]/children[0]/x: number mismatch (12.5 != 12.0)
- `tickets/layered/701_portLabels.elkt`: diffs=20 [coordinate=16, label=4], first: children[0]/children[0]/children[0]/labels[0]/x: number mismatch (38.0 != 5.0)
