# ELK Model Parity Report

- manifest: `perf/model_parity_step48_section_semantics/rust_manifest.tsv`
- total rows: 4
- compared rows: 4
- matched rows: 2
- drift rows: 2
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 5

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| section | 3 | 60.0% |
| structure | 2 | 40.0% |

### Top Diff Path Prefixes

- `children[*]/edges[*]/sections[*]`: 2 (40.0%)
- `edges[*]/sections[*]/startPoint`: 2 (40.0%)
- `edges[*]/sections[*]/endPoint`: 1 (20.0%)

## Drift Samples

- `tests/layered/connected_components/compound06.elkt`: diffs=2 [structure=2], first: children[0]/edges[4]/sections[0]: missing keys on right: bendPoints
- `tickets/layered/724_includeChildrenModelOrder.elkt`: diffs=3 [section=3], first: edges[0]/sections[0]/endPoint/y: number mismatch (27.5 != 27.0)
