# ELK Model Parity Report

- manifest: `/Users/cody.ij.hwang/Projects/github/elk-rs/perf/model_parity/rust_manifest.tsv`
- total rows: 1448
- compared rows: 1439
- matched rows: 1436
- drift rows: 3
- skipped rows (java/rust non-ok): 9
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 31

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 28 | 90.3% |
| other | 2 | 6.5% |
| section | 1 | 3.2% |

### Top Diff Path Prefixes

- `children[*]/children[*]/y`: 17 (54.8%)
- `children[*]/ports[*]/x`: 5 (16.1%)
- `children[*]/children[*]/x`: 3 (9.7%)
- `children[*]/width`: 2 (6.5%)
- `width`: 2 (6.5%)
- `children[*]/x`: 1 (3.2%)
- `children[*]/edges[*]/sections[*]`: 1 (3.2%)

## Drift Samples

- `tests/core/label_placement/port_labels/next_to_port_if_possible_inside.elkt`: diffs=5 [coordinate=4, other=1], first: children[0]/ports[0]/x: number mismatch (4.0 != 24.0)
- `tests/layered/port_label_placement/multilabels_compound.elkt`: diffs=6 [coordinate=4, section=1, other=1], first: children[0]/x: number mismatch (38.0 != 58.0)
- `tickets/layered/213_componentsCompaction.elkt`: diffs=20 [coordinate=20], first: children[0]/children[0]/x: number mismatch (12.5 != 12.0)
