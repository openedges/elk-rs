# ELK Model Parity Report

- manifest: `/Users/cody.ij.hwang/Projects/github/elk-rs/parity/model_parity_tickets/rust_manifest.tsv`
- total rows: 110
- compared rows: 109
- matched rows: 108
- drift rows: 1
- skipped rows (java/rust non-ok): 1
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 20

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 20 | 100.0% |

### Top Diff Path Prefixes

- `children[*]/children[*]/y`: 17 (85.0%)
- `children[*]/children[*]/x`: 3 (15.0%)

## Drift Samples

- `tickets/layered/213_componentsCompaction.elkt`: diffs=20 [coordinate=20], first: children[0]/children[0]/x: number mismatch (12.5 != 12.0)
