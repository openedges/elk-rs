# ELK Model Parity Report

- manifest: `perf/model_parity_step51_phase1_off/rust_manifest.tsv`
- total rows: 10
- compared rows: 10
- matched rows: 0
- drift rows: 10
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 132

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 80 | 60.6% |
| section | 40 | 30.3% |
| label | 6 | 4.5% |
| other | 5 | 3.8% |
| structure | 1 | 0.8% |

### Top Diff Path Prefixes

- `children[*]/children[*]/children[*]`: 24 (18.2%)
- `edges[*]/sections[*]/bendPoints[*]`: 23 (17.4%)
- `children[*]/ports[*]/x`: 12 (9.1%)
- `children[*]/children[*]/y`: 11 (8.3%)
- `children[*]/children[*]/x`: 9 (6.8%)
- `children[*]/children[*]/edges[*]`: 9 (6.8%)
- `children[*]/ports[*]/y`: 8 (6.1%)
- `children[*]/height`: 5 (3.8%)
- `children[*]/y`: 4 (3.0%)
- `edges[*]/sections[*]/endPoint`: 4 (3.0%)

## Drift Samples

- `tickets/layered/182_minNodeSizeForHierarchicalNodes.elkt`: diffs=8 [coordinate=7, other=1], first: children[0]/ports[0]/x: number mismatch (6.285714285714286 != 0.0)
- `tickets/layered/213_componentsCompaction.elkt`: diffs=20 [coordinate=20], first: children[0]/children[0]/x: number mismatch (12.5 != 12.0)
- `tickets/layered/302_brokenSplineSelfLoops.elkt`: diffs=20 [section=19, coordinate=1], first: children[0]/y: number mismatch (12.0 != 23.333333333333336)
- `tickets/layered/352_selfLoopNPEorAIOOBE.elkt`: diffs=6 [section=6], first: edges[4]/sections[0]/bendPoints[0]/y: number mismatch (64.0 != 118.0)
- `tickets/layered/368_selfLoopLabelsIOOBE.elkt`: diffs=8 [section=6, coordinate=1, label=1], first: children[0]/x: number mismatch (22.0 != 12.0)
- `tickets/layered/425_selfLoopInCompoundNode.elkt`: diffs=8 [section=4, coordinate=3, other=1], first: children[0]/children[0]/edges[0]/sections[0]/bendPoints[0]/x: number mismatch (15.0 != 5.0)
- `tickets/layered/453_interactiveProblems.elkt`: diffs=20 [coordinate=18, label=2], first: children[0]/height: number mismatch (61.0 != 45.0)
- `tickets/layered/502_collapsingCompoundNode.elkt`: diffs=7 [coordinate=5, section=1, other=1], first: children[0]/edges[0]/sections[0]/startPoint/y: number mismatch (4.0 != 24.0)
- `tickets/layered/665_includeChildrenDoesntStop.elkt`: diffs=15 [coordinate=8, section=4, other=2, structure=1], first: children[0]/children[0]/children[0]/x: number mismatch (149.4242331510003 != 12.0)
- `tickets/layered/701_portLabels.elkt`: diffs=20 [coordinate=17, label=3], first: children[0]/children[0]/children[0]/height: number mismatch (52.0 != 44.0)
