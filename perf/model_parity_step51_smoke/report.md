# ELK Model Parity Report

- manifest: `perf/model_parity_step51_smoke/rust_manifest.tsv`
- total rows: 16
- compared rows: 16
- matched rows: 7
- drift rows: 9
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 120

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 67 | 55.8% |
| section | 44 | 36.7% |
| other | 4 | 3.3% |
| label | 4 | 3.3% |
| structure | 1 | 0.8% |

### Top Diff Path Prefixes

- `edges[*]/sections[*]/bendPoints[*]`: 27 (22.5%)
- `children[*]/children[*]/children[*]`: 24 (20.0%)
- `children[*]/children[*]/y`: 11 (9.2%)
- `children[*]/children[*]/x`: 9 (7.5%)
- `children[*]/ports[*]/y`: 8 (6.7%)
- `edges[*]/sections[*]/endPoint`: 7 (5.8%)
- `children[*]/ports[*]/x`: 6 (5.0%)
- `edges[*]/sections[*]/startPoint`: 5 (4.2%)
- `children[*]/height`: 5 (4.2%)
- `children[*]/children[*]/edges[*]`: 5 (4.2%)

## Drift Samples

- `tickets/layered/182_minNodeSizeForHierarchicalNodes.elkt`: diffs=8 [coordinate=7, other=1], first: children[0]/ports[0]/x: number mismatch (6.285714285714286 != 10.0)
- `tickets/layered/213_componentsCompaction.elkt`: diffs=20 [coordinate=20], first: children[0]/children[0]/x: number mismatch (12.5 != 12.0)
- `tickets/layered/302_brokenSplineSelfLoops.elkt`: diffs=20 [section=19, coordinate=1], first: children[0]/y: number mismatch (12.0 != 23.333333333333336)
- `tickets/layered/352_selfLoopNPEorAIOOBE.elkt`: diffs=6 [section=6], first: edges[4]/sections[0]/bendPoints[0]/y: number mismatch (64.0 != 118.0)
- `tickets/layered/368_selfLoopLabelsIOOBE.elkt`: diffs=8 [section=6, coordinate=1, label=1], first: children[0]/x: number mismatch (22.0 != 12.0)
- `tickets/layered/453_interactiveProblems.elkt`: diffs=16 [coordinate=8, section=8], first: children[0]/height: number mismatch (61.0 != 38.69999980926514)
- `tickets/layered/502_collapsingCompoundNode.elkt`: diffs=7 [coordinate=5, section=1, other=1], first: children[0]/edges[0]/sections[0]/startPoint/y: number mismatch (4.0 != 24.0)
- `tickets/layered/665_includeChildrenDoesntStop.elkt`: diffs=15 [coordinate=8, section=4, other=2, structure=1], first: children[0]/children[0]/children[0]/x: number mismatch (149.4242331510003 != 12.0)
- `tickets/layered/701_portLabels.elkt`: diffs=20 [coordinate=17, label=3], first: children[0]/children[0]/children[0]/height: number mismatch (52.0 != 44.0)
