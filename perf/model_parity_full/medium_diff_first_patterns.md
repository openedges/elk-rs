# Medium Diff First-Pattern Grouping

- source: `perf/model_parity_full/diff_details.tsv`
- medium range: diff_count 6..19
- medium model count: 84

## Top First-Diff Patterns

| rank | first_path_norm | count |
|---:|---|---:|
| 1 | `children[*]/y` | 28 |
| 2 | `children[*]/children[*]/x` | 10 |
| 3 | `children[*]/height` | 8 |
| 4 | `children[*]/children[*]/y` | 7 |
| 5 | `children[*]/x` | 7 |
| 6 | `children[*]/ports[*]/y` | 4 |
| 7 | `edges[*]/labels[*]/y` | 4 |
| 8 | `children[*]/ports[*]/x` | 3 |
| 9 | `children[*]/children[*]/labels[*]/x` | 2 |
| 10 | `children[*]/edges[*]/sections[*]` | 2 |
| 11 | `children[*]/edges[*]/sections[*]/startPoint/x` | 2 |
| 12 | `children[*]/children[*]/children[*]/x` | 1 |
| 13 | `children[*]/children[*]/height` | 1 |
| 14 | `children[*]/children[*]/ports[*]/x` | 1 |
| 15 | `children[*]/children[*]/ports[*]/y` | 1 |
| 16 | `children[*]/edges[*]/sections[*]/bendPoints[*]/y` | 1 |
| 17 | `children[*]/edges[*]/sections[*]/endPoint/x` | 1 |
| 18 | `children[*]/ports[*]/labels[*]/y` | 1 |

## Top Categories

| category | count |
|---|---:|
| `section` | 42 |
| `coordinate` | 34 |
| `label` | 8 |

## Prefix Breakdown

| prefix | count |
|---|---:|
| `tests` | 39 |
| `tickets` | 32 |
| `realworld` | 10 |
| `examples` | 3 |

## Representative Models (top 10 patterns, first 3 each)

- `children[*]/y`
  - `tickets/layered/128_selfLoopLabelSpacing_simple.elkt` (diff=7, top=section, first=`children[0]/y`)
  - `tests/layered/node_placement/bk/classes/classes.elkt` (diff=8, top=section, first=`children[0]/y`)
  - `tests/layered/edge_label_placement/layerSelection_center_01.elkt` (diff=9, top=section, first=`children[7]/y`)
- `children[*]/children[*]/x`
  - `tests/layered/hierarchical_ports/hierarchy03.elkt` (diff=8, top=coordinate, first=`children[0]/children[0]/x`)
  - `tickets/layered/404_stackedCenterEdgeLabels.elkt` (diff=8, top=coordinate, first=`children[0]/children[0]/x`)
  - `tickets/layered/546_borderGaps.elkt` (diff=10, top=coordinate, first=`children[0]/children[0]/x`)
- `children[*]/height`
  - `tests/core/label_placement/port_labels/treat_as_group_outside.elkt` (diff=6, top=coordinate, first=`children[0]/height`)
  - `tests/layered/node_placement/flexible_ports/graph01.elkt` (diff=10, top=coordinate, first=`children[0]/height`)
  - `tickets/layered/297_sameSideInsideSelfLoop.elkt` (diff=11, top=coordinate, first=`children[0]/height`)
- `children[*]/children[*]/y`
  - `examples/user-hints/layered/verticalOrder.elkt` (diff=9, top=coordinate, first=`children[1]/children[1]/y`)
  - `tests/layered/self_loops/inside_outside.elkt` (diff=13, top=section, first=`children[0]/children[0]/y`)
  - `tickets/layered/455_shortHierarchicalEdges.elkt` (diff=14, top=coordinate, first=`children[0]/children[0]/y`)
- `children[*]/x`
  - `tickets/layered/471_verticalInlineLabelAlignment.elkt` (diff=7, top=coordinate, first=`children[0]/x`)
  - `tests/layered/labels/edge_label_side_selection/smart_04.elkt` (diff=9, top=section, first=`children[0]/x`)
  - `tickets/layered/302_brokenSplineSelfLoops.elkt` (diff=9, top=section, first=`children[0]/x`)
- `children[*]/ports[*]/y`
  - `tickets/core/562_insideSelfLoopAlgorithmResolving.elkt` (diff=6, top=coordinate, first=`children[0]/ports[0]/y`)
  - `examples/labels/portLabelsMulti.elkt` (diff=8, top=coordinate, first=`children[0]/ports[2]/y`)
  - `tests/layered/port_label_placement/multilabels.elkt` (diff=8, top=coordinate, first=`children[0]/ports[2]/y`)
- `edges[*]/labels[*]/y`
  - `tests/layered/interactive_layout/noDummyNodes.elkt` (diff=8, top=label, first=`edges[0]/labels[0]/y`)
  - `tickets/layered/360_badSelfLoopLabelPlacement.elkt` (diff=9, top=section, first=`edges[1]/labels[0]/y`)
  - `tests/layered/edge_label_placement/layerSelection_center_02.elkt` (diff=14, top=label, first=`edges[2]/labels[0]/y`)
- `children[*]/ports[*]/x`
  - `tests/layered/node_placement/compact/unbalanced.elkt` (diff=10, top=coordinate, first=`children[1]/ports[0]/x`)
  - `tests/layered/north_south_ports/nsp2.elkt` (diff=16, top=section, first=`children[3]/ports[0]/x`)
  - `tickets/layered/444_selfLoopsNPE.elkt` (diff=19, top=section, first=`children[0]/ports[0]/x`)
- `children[*]/children[*]/labels[*]/x`
  - `tickets/layered/905_beginLabelAboveEdge.elkt` (diff=11, top=label, first=`children[0]/children[0]/labels[0]/x`)
  - `tickets/layered/905_beginLableBellowEdge.elkt` (diff=11, top=label, first=`children[0]/children[0]/labels[0]/x`)
- `children[*]/edges[*]/sections[*]`
  - `tickets/layered/193_hierarchicalPortOverlaps.elkt` (diff=7, top=coordinate, first=`children[0]/edges[0]/sections[0]`)
  - `tests/layered/hierarchical_ports/hierarchy04.elkt` (diff=16, top=section, first=`children[0]/edges[0]/sections[0]`)
