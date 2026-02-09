# ELK Model Parity Report

- manifest: `perf/model_parity/rust_manifest.tsv`
- total rows: 100
- compared rows: 96
- matched rows: 20
- drift rows: 76
- skipped rows (java/rust non-ok): 4
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 1417

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 1147 | 80.9% |
| section | 219 | 15.5% |
| structure | 33 | 2.3% |
| other | 11 | 0.8% |
| label | 5 | 0.4% |
| ordering | 2 | 0.1% |

### Top Diff Path Prefixes

- `children[*]/y`: 766 (54.1%)
- `children[*]/x`: 300 (21.2%)
- `edges[*]/sections[*]/endPoint`: 62 (4.4%)
- `edges[*]/sections[*]/bendPoints[*]`: 56 (4.0%)
- `children[*]/edges[*]/sections[*]`: 51 (3.6%)
- `edges[*]/sections[*]/startPoint`: 48 (3.4%)
- `children[*]/children[*]/x`: 29 (2.0%)
- `children[*]/children[*]/y`: 27 (1.9%)
- `edges[*]/sections[*]`: 14 (1.0%)
- `children[*]/ports[*]/y`: 12 (0.8%)

## Drift Samples

- `examples/hierarchy/hierarchicalEdges.elkt`: diffs=9 [coordinate=6, other=2, structure=1], first: children[0]/children[0]/x: number mismatch (17.0 != 12.0)
- `examples/hierarchy/hierarchicalLayoutMixing.elkt`: diffs=20 [coordinate=10, section=8, structure=2], first: children[0]/children[0]/x: number mismatch (17.0 != 44.558860981188595)
- `examples/labels/portLabelsMulti.elkt`: diffs=4 [label=4], first: children[1]/ports[2]/labels[0]/y: number mismatch (1.0 != -31.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_circle.elkt`: diffs=18 [section=12, coordinate=4, structure=1, other=1], first: children[0]/y: number mismatch (12.0 != 17.5)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_hierarchyDirection.elkt`: diffs=20 [section=12, coordinate=7, structure=1], first: children[0]/x: number mismatch (52.0 != 62.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_hierarchyDirection_pseudo_positions.elkt`: diffs=20 [section=12, coordinate=7, structure=1], first: children[0]/x: number mismatch (52.0 != 62.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_hierarchy_pseudo_positions.elkt`: diffs=8 [coordinate=4, section=4], first: children[0]/x: number mismatch (76.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_motor.elkt`: diffs=20 [section=11, coordinate=9], first: children[0]/x: number mismatch (82.0 != 72.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_separateComponents_pseudo_positions.elkt`: diffs=8 [coordinate=3, section=3, other=2], first: children[0]/y: number mismatch (32.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_simpleDirectionTest.elkt`: diffs=16 [section=12, coordinate=4], first: children[0]/y: number mismatch (12.0 != 32.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_simpleDirectionTest_pseudo_positions.elkt`: diffs=16 [section=12, coordinate=4], first: children[0]/y: number mismatch (12.0 != 32.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_snake_pseudo_positions.elkt`: diffs=20 [coordinate=20], first: children[1]/x: number mismatch (132.0 != 192.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_sortingTask.elkt`: diffs=20 [section=10, coordinate=9, structure=1], first: children[0]/x: number mismatch (62.0 != 72.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_sortingTask_pseudo_positions.elkt`: diffs=20 [coordinate=12, section=6, structure=2], first: children[0]/x: number mismatch (12.0 != 82.0)
- `examples/user-hints/interactive-constraints/interactiveLayout_mixedHierarchy.elkt`: diffs=8 [coordinate=6, other=2], first: children[0]/x: number mismatch (15.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveRectpacking_hierarchy.elkt`: diffs=12 [coordinate=10, other=2], first: children[0]/x: number mismatch (15.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveRectpacking_oneBigNode.elkt`: diffs=16 [coordinate=14, other=2], first: children[0]/x: number mismatch (15.0 != 12.0)
- `examples/user-hints/layered/horizontalOrder.elkt`: diffs=20 [coordinate=10, section=8, structure=2], first: children[1]/children[0]/x: number mismatch (32.0 != 12.0)
- `examples/user-hints/layered/partitioning.elkt`: diffs=20 [coordinate=18, section=2], first: children[0]/children[0]/y: number mismatch (86.0 != 96.0)
- `examples/user-hints/layered/reverseEdge.elkt`: diffs=20 [section=12, coordinate=6, structure=2], first: children[0]/children[0]/x: number mismatch (52.0 != 72.0)
