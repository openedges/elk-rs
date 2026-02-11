# ELK Model Parity Report

- manifest: `/Users/cody.ij.hwang/Projects/research/ip-integrator/elk-rs/perf/model_parity_examples/rust_manifest.tsv`
- total rows: 45
- compared rows: 33
- matched rows: 16
- drift rows: 17
- skipped rows (java/rust non-ok): 12
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 265

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 124 | 46.8% |
| section | 115 | 43.4% |
| structure | 10 | 3.8% |
| other | 9 | 3.4% |
| label | 5 | 1.9% |
| ordering | 2 | 0.8% |

### Top Diff Path Prefixes

- `children[*]/edges[*]/sections[*]`: 40 (15.1%)
- `children[*]/y`: 33 (12.5%)
- `children[*]/x`: 29 (10.9%)
- `edges[*]/sections[*]/bendPoints[*]`: 26 (9.8%)
- `edges[*]/sections[*]/endPoint`: 25 (9.4%)
- `children[*]/children[*]/y`: 25 (9.4%)
- `edges[*]/sections[*]/startPoint`: 22 (8.3%)
- `children[*]/children[*]/x`: 21 (7.9%)
- `children[*]/children[*]/edges[*]`: 8 (3.0%)
- `children[*]/children[*]/children[*]`: 6 (2.3%)

## Drift Samples

- `examples/edges/insideSelfLoops.elkt`: diffs=18 [section=16, coordinate=2], first: children[0]/y: number mismatch (22.0 != 12.0)
- `examples/hierarchy/hierarchicalEdges.elkt`: diffs=11 [coordinate=8, other=2, structure=1], first: children[0]/children[0]/height: number mismatch (24.0 != 0.0)
- `examples/hierarchy/hierarchicalLayoutMixing.elkt`: diffs=20 [coordinate=12, section=6, structure=2], first: children[0]/children[0]/height: number mismatch (24.0 != 0.0)
- `examples/labels/portLabelsMulti.elkt`: diffs=4 [label=4], first: children[1]/ports[2]/labels[0]/y: number mismatch (1.0 != -31.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_circle.elkt`: diffs=18 [section=12, coordinate=4, structure=1, other=1], first: children[0]/y: number mismatch (12.0 != 17.5)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_hierarchyDirection.elkt`: diffs=20 [section=12, coordinate=7, structure=1], first: children[0]/x: number mismatch (52.0 != 62.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_motor.elkt`: diffs=20 [section=11, coordinate=9], first: children[0]/x: number mismatch (82.0 != 72.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_simpleDirectionTest.elkt`: diffs=16 [section=12, coordinate=4], first: children[0]/y: number mismatch (12.0 != 32.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_sortingTask.elkt`: diffs=20 [section=10, coordinate=9, structure=1], first: children[0]/x: number mismatch (62.0 != 72.0)
- `examples/user-hints/interactive-constraints/interactiveLayout_mixedHierarchy.elkt`: diffs=8 [coordinate=6, other=2], first: children[0]/x: number mismatch (15.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveRectpacking_hierarchy.elkt`: diffs=12 [coordinate=10, other=2], first: children[0]/x: number mismatch (15.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveRectpacking_oneBigNode.elkt`: diffs=16 [coordinate=14, other=2], first: children[0]/x: number mismatch (15.0 != 12.0)
- `examples/user-hints/layered/partitioning.elkt`: diffs=20 [coordinate=18, section=2], first: children[0]/children[0]/y: number mismatch (86.0 != 96.0)
- `examples/user-hints/layered/reverseEdge.elkt`: diffs=20 [section=12, coordinate=6, structure=2], first: children[0]/children[0]/x: number mismatch (52.0 != 72.0)
- `examples/user-hints/layered/verticalOrder.elkt`: diffs=20 [coordinate=9, section=9, structure=1, label=1], first: children[0]/children[0]/y: number mismatch (32.0 != 42.0)
- `examples/user-hints/model-order/modelOrderCycleBreaking.elkt`: diffs=20 [section=13, coordinate=6, structure=1], first: children[0]/children[0]/y: number mismatch (23.0 != 12.0)
- `examples/user-hints/model-order/modelOrderNoCrossingMin.elkt`: diffs=2 [ordering=2], first: edges[2]/sections[0]/bendPoints: array length mismatch (2 != 4)
