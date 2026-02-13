# ELK Model Parity Report

- manifest: `/Users/cody.ij.hwang/Projects/github/elk-rs/perf/model_parity_tickets/rust_manifest.tsv`
- total rows: 110
- compared rows: 109
- matched rows: 42
- drift rows: 67
- skipped rows (java/rust non-ok): 1
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 860

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| section | 368 | 42.8% |
| coordinate | 356 | 41.4% |
| other | 48 | 5.6% |
| label | 43 | 5.0% |
| structure | 32 | 3.7% |
| ordering | 13 | 1.5% |

### Top Diff Path Prefixes

- `children[*]/edges[*]/sections[*]`: 147 (17.1%)
- `edges[*]/sections[*]/endPoint`: 82 (9.5%)
- `children[*]/y`: 81 (9.4%)
- `edges[*]/sections[*]/bendPoints[*]`: 77 (9.0%)
- `edges[*]/sections[*]/startPoint`: 67 (7.8%)
- `children[*]/ports[*]/y`: 63 (7.3%)
- `children[*]/x`: 40 (4.7%)
- `children[*]/children[*]/y`: 37 (4.3%)
- `children[*]/children[*]/x`: 33 (3.8%)
- `width`: 27 (3.1%)

## Drift Samples

- `tickets/core/056_portLabelPlacement.elkt`: diffs=2 [coordinate=2], first: children[0]/ports[0]/y: number mismatch (55.0 != 35.0)
- `tickets/core/299_surroundingPortSpace.elkt`: diffs=1 [coordinate=1], first: children[0]/height: number mismatch (40.0 != 20.0)
- `tickets/core/491_portSpacing.elkt`: diffs=4 [coordinate=2, section=2], first: children[0]/ports[1]/y: number mismatch (37.0 != 38.0)
- `tickets/core/562_insideSelfLoopAlgorithmResolving.elkt`: diffs=6 [coordinate=3, section=3], first: children[0]/ports[0]/y: number mismatch (12.0 != 0.0)
- `tickets/layered/079_selfLoopLabels.elkt`: diffs=1 [ordering=1], first: edges[0]/sections[0]/bendPoints: array length mismatch (4 != 2)
- `tickets/layered/082_unfortunateEdgeLabelSideSelection.elkt`: diffs=20 [section=14, label=4, coordinate=2], first: children[1]/x: number mismatch (15.0 != 12.0)
- `tickets/layered/128_selfLoopLabelSpacing_complex.elkt`: diffs=20 [section=10, label=7, coordinate=3], first: children[0]/y: number mismatch (192.0 != 120.0)
- `tickets/layered/128_selfLoopLabelSpacing_simple.elkt`: diffs=7 [section=4, coordinate=1, label=1, other=1], first: children[0]/y: number mismatch (52.0 != 34.0)
- `tickets/layered/167_insidePortLabelsInHierarchicalGraphs.elkt`: diffs=4 [coordinate=2, section=1, other=1], first: children[1]/edges[1]/sections[0]/endPoint/x: number mismatch (4.0 != 24.0)
- `tickets/layered/193_hierarchicalPortOverlaps.elkt`: diffs=7 [coordinate=4, structure=1, section=1, other=1], first: children[0]/edges[0]/sections[0]: missing keys on right: bendPoints
- `tickets/layered/194_excessiveWhiteSpace.elkt`: diffs=20 [section=9, coordinate=8, ordering=2, structure=1], first: children[0]/children[0]/y: number mismatch (153.0 != 72.0)
- `tickets/layered/213_componentsCompaction.elkt`: diffs=20 [coordinate=20], first: children[0]/children[0]/x: number mismatch (12.5 != 12.0)
- `tickets/layered/273_insideSelfLoopsWithLabels.elkt`: diffs=15 [coordinate=7, section=4, label=2, other=2], first: children[0]/children[0]/ports[0]/x: number mismatch (0.0 != -20.0)
- `tickets/layered/288_selfLoopsCauseNPE_a.elkt`: diffs=17 [section=9, coordinate=5, other=2, ordering=1], first: children[0]/y: number mismatch (24.0 != 49.0)
- `tickets/layered/288_selfLoopsCauseNPE_b.elkt`: diffs=20 [section=11, coordinate=7, ordering=1, structure=1], first: children[0]/children[0]/children[0]/y: number mismatch (22.0 != 32.0)
- `tickets/layered/297_sameSideInsideSelfLoop.elkt`: diffs=11 [coordinate=4, section=4, other=2, structure=1], first: children[0]/height: number mismatch (74.0 != 100.0)
- `tickets/layered/298_selfLoopsCauseAIOOBE.elkt`: diffs=14 [coordinate=7, section=4, other=2, structure=1], first: children[0]/height: number mismatch (44.0 != 100.0)
- `tickets/layered/302_brokenSplineSelfLoops.elkt`: diffs=6 [section=2, other=2, coordinate=1, ordering=1], first: children[0]/x: number mismatch (23.333333333333332 != 27.0)
- `tickets/layered/304_wrongMinimumNodeSize.elkt`: diffs=20 [coordinate=10, section=8, structure=2], first: children[0]/y: number mismatch (47.0 != 12.0)
- `tickets/layered/308_hyperedgeMerging.elkt`: diffs=17 [section=12, coordinate=2, structure=1, ordering=1, other=1], first: children[1]/y: number mismatch (23.0 != 33.0)
