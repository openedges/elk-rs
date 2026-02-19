# ELK Model Parity Report

- manifest: `perf/model_parity_step51_after_import_fix/rust_manifest.tsv`
- total rows: 10
- compared rows: 10
- matched rows: 2
- drift rows: 8
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 112

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 60 | 53.6% |
| section | 44 | 39.3% |
| label | 4 | 3.6% |
| other | 3 | 2.7% |
| structure | 1 | 0.9% |

### Top Diff Path Prefixes

- `edges[*]/sections[*]/bendPoints[*]`: 27 (24.1%)
- `children[*]/children[*]/children[*]`: 24 (21.4%)
- `children[*]/children[*]/y`: 11 (9.8%)
- `children[*]/children[*]/x`: 9 (8.0%)
- `children[*]/ports[*]/y`: 8 (7.1%)
- `edges[*]/sections[*]/endPoint`: 7 (6.2%)
- `edges[*]/sections[*]/startPoint`: 5 (4.5%)
- `children[*]/height`: 5 (4.5%)
- `children[*]/children[*]/edges[*]`: 5 (4.5%)
- `children[*]/y`: 2 (1.8%)

## Drift Samples

- `tickets/layered/213_componentsCompaction.elkt`: diffs=20 [coordinate=20], first: children[0]/children[0]/x: number mismatch (12.5 != 12.0)
- `tickets/layered/302_brokenSplineSelfLoops.elkt`: diffs=20 [section=19, coordinate=1], first: children[0]/y: number mismatch (12.0 != 23.333333333333336)
- `tickets/layered/352_selfLoopNPEorAIOOBE.elkt`: diffs=6 [section=6], first: edges[4]/sections[0]/bendPoints[0]/y: number mismatch (64.0 != 118.0)
- `tickets/layered/368_selfLoopLabelsIOOBE.elkt`: diffs=8 [section=6, coordinate=1, label=1], first: children[0]/x: number mismatch (22.0 != 12.0)
- `tickets/layered/453_interactiveProblems.elkt`: diffs=16 [coordinate=8, section=8], first: children[0]/height: number mismatch (61.0 != 38.69999980926514)
- `tickets/layered/502_collapsingCompoundNode.elkt`: diffs=7 [coordinate=5, section=1, other=1], first: children[0]/edges[0]/sections[0]/startPoint/y: number mismatch (4.0 != 24.0)
- `tickets/layered/665_includeChildrenDoesntStop.elkt`: diffs=15 [coordinate=8, section=4, other=2, structure=1], first: children[0]/children[0]/children[0]/x: number mismatch (149.4242331510003 != 12.0)
- `tickets/layered/701_portLabels.elkt`: diffs=20 [coordinate=17, label=3], first: children[0]/children[0]/children[0]/height: number mismatch (52.0 != 44.0)
