# ELK Model Parity Report

- manifest: `perf/model_parity/rust_manifest.tsv`
- total rows: 100
- compared rows: 96
- matched rows: 13
- drift rows: 83
- skipped rows (java/rust non-ok): 4
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 1539

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 1229 | 79.9% |
| section | 254 | 16.5% |
| structure | 36 | 2.3% |
| other | 13 | 0.8% |
| label | 5 | 0.3% |
| ordering | 2 | 0.1% |

### Top Diff Path Prefixes

- `children[*]/y`: 782 (50.8%)
- `children[*]/x`: 302 (19.6%)
- `edges[*]/sections[*]/endPoint`: 76 (4.9%)
- `edges[*]/sections[*]/bendPoints[*]`: 66 (4.3%)
- `children[*]/edges[*]/sections[*]`: 63 (4.1%)
- `edges[*]/sections[*]/startPoint`: 48 (3.1%)
- `children[*]/ports[*]/y`: 37 (2.4%)
- `children[*]/children[*]/x`: 30 (1.9%)
- `children[*]/children[*]/y`: 29 (1.9%)
- `children[*]/ports[*]/x`: 18 (1.2%)

## Drift Samples

- `examples/general/mixingDirection.elkt`: diffs=4 [section=3, coordinate=1], first: children[0]/children[0]/x: number mismatch (22.0 != 12.0)
- `examples/general/spacing/ports.elkt`: diffs=20 [coordinate=20], first: children[0]/ports[0]/x: number mismatch (16.666666666666668 != 0.0)
- `examples/general/spacing/portsSurrounding.elkt`: diffs=20 [coordinate=20], first: children[0]/ports[0]/x: number mismatch (100.0 != 0.0)
- `examples/hierarchy/hierarchicalEdges.elkt`: diffs=9 [coordinate=6, other=2, structure=1], first: children[0]/children[0]/x: number mismatch (17.0 != 12.0)
- `examples/hierarchy/hierarchicalLayoutMixing.elkt`: diffs=20 [coordinate=10, section=8, structure=2], first: children[0]/children[0]/x: number mismatch (17.0 != 44.558860981188595)
- `examples/labels/portLabelsMulti.elkt`: diffs=20 [coordinate=15, label=4, other=1], first: children[0]/ports[0]/x: number mismatch (170.0 != 0.0)
- `examples/ports/portConstraints.elkt`: diffs=20 [coordinate=10, section=8, structure=1, other=1], first: children[0]/children[0]/y: number mismatch (12.0 != 37.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_circle.elkt`: diffs=18 [section=12, coordinate=4, structure=1, other=1], first: children[0]/y: number mismatch (12.0 != 17.5)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_hierarchy.elkt`: diffs=3 [section=2, coordinate=1], first: children[0]/y: number mismatch (24.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_hierarchyDirection.elkt`: diffs=20 [coordinate=11, section=8, structure=1], first: children[0]/x: number mismatch (52.0 != 62.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_hierarchyDirection_pseudo_positions.elkt`: diffs=20 [coordinate=11, section=8, structure=1], first: children[0]/x: number mismatch (52.0 != 62.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_hierarchy_pseudo_positions.elkt`: diffs=11 [section=6, coordinate=5], first: children[0]/x: number mismatch (76.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_motor.elkt`: diffs=20 [section=11, coordinate=9], first: children[0]/x: number mismatch (82.0 != 72.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_separateComponents_pseudo_positions.elkt`: diffs=8 [coordinate=3, section=3, other=2], first: children[0]/y: number mismatch (32.0 != 12.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_simpleDirectionTest.elkt`: diffs=16 [section=12, coordinate=4], first: children[0]/y: number mismatch (12.0 != 32.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_simpleDirectionTest_pseudo_positions.elkt`: diffs=16 [section=12, coordinate=4], first: children[0]/y: number mismatch (12.0 != 32.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_snake_pseudo_positions.elkt`: diffs=20 [coordinate=20], first: children[1]/x: number mismatch (132.0 != 192.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_sortingTask.elkt`: diffs=20 [section=10, coordinate=9, structure=1], first: children[0]/x: number mismatch (62.0 != 72.0)
- `examples/user-hints/interactive-constraints/interactiveLayeredLayout_sortingTask_pseudo_positions.elkt`: diffs=20 [coordinate=12, section=6, structure=2], first: children[0]/x: number mismatch (12.0 != 82.0)
- `examples/user-hints/interactive-constraints/interactiveLayout_mixedHierarchy.elkt`: diffs=8 [coordinate=6, other=2], first: children[0]/x: number mismatch (15.0 != 12.0)
