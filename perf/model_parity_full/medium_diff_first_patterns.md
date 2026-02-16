# Medium Diff First-Pattern Grouping

- source: `perf/model_parity_full/diff_details.tsv`
- medium range: diff_count 6..19
- medium model count: 37

## Top First-Diff Patterns

| rank | first_path_norm | count |
|---:|---|---:|
| 1 | `children[*]/children[*]/children[*]/y` | 6 |
| 2 | `children[*]/children[*]/y` | 6 |
| 3 | `children[*]/x` | 5 |
| 4 | `children[*]/ports[*]/x` | 4 |
| 5 | `children[*]/y` | 4 |
| 6 | `children[*]/height` | 3 |
| 7 | `edges[*]/labels[*]/y` | 2 |
| 8 | `children[*]/children[*]/children[*]/x` | 1 |
| 9 | `children[*]/children[*]/ports[*]/x` | 1 |
| 10 | `children[*]/children[*]/ports[*]/y` | 1 |
| 11 | `children[*]/edges[*]/sections[*]/startPoint/y` | 1 |
| 12 | `children[*]/ports[*]/labels[*]/y` | 1 |
| 13 | `children[*]/ports[*]/y` | 1 |
| 14 | `edges[*]/sections[*]/bendPoints[*]/y` | 1 |

## Top Categories

| category | count |
|---|---:|
| `section` | 22 |
| `coordinate` | 12 |
| `label` | 2 |
| `ordering` | 1 |

## Prefix Breakdown

| prefix | count |
|---|---:|
| `tickets` | 14 |
| `realworld` | 12 |
| `tests` | 9 |
| `examples` | 2 |

## Representative Models (top 10 patterns, first 3 each)

- `children[*]/children[*]/children[*]/y`
  - `realworld/ptolemy/hierarchical/continuous_cartpendulum_CartPendulum.elkg` (diff=14, top=section, first=`children[2]/children[8]/children[0]/y`)
  - `realworld/ptolemy/hierarchical/continuous_cartpendulum_CartPendulum.elkt` (diff=14, top=section, first=`children[2]/children[8]/children[0]/y`)
  - `realworld/ptolemy/hierarchical/continuous_pendulum3d_Pendulum3D.elkg` (diff=14, top=section, first=`children[4]/children[7]/children[0]/y`)
- `children[*]/children[*]/y`
  - `examples/user-hints/layered/verticalOrder.elkt` (diff=9, top=coordinate, first=`children[1]/children[1]/y`)
  - `tests/layered/self_loops/inside_outside.elkt` (diff=15, top=section, first=`children[0]/children[0]/y`)
  - `realworld/ptolemy/hierarchical/ptolemy_brewery_Brewery.elkg` (diff=18, top=section, first=`children[3]/children[1]/y`)
- `children[*]/x`
  - `tests/layered/compaction_oned/selfloop/selfloop_crash.elkt` (diff=6, top=ordering, first=`children[4]/x`)
  - `tests/layered/compaction_oned/labels/edgeLabelAndSplines.elkt` (diff=12, top=section, first=`children[2]/x`)
  - `tickets/layered/515_polylineOverNodeOutgoing.elkt` (diff=14, top=section, first=`children[0]/x`)
- `children[*]/ports[*]/x`
  - `tickets/layered/182_minNodeSizeForHierarchicalNodes.elkt` (diff=8, top=coordinate, first=`children[0]/ports[0]/x`)
  - `tickets/core/562_insideSelfLoopAlgorithmResolving.elkt` (diff=10, top=section, first=`children[0]/ports[1]/x`)
  - `tickets/layered/298_selfLoopsCauseAIOOBE.elkt` (diff=10, top=section, first=`children[0]/ports[1]/x`)
- `children[*]/y`
  - `tests/layered/node_placement/bk/classes/classes_two_samesize.elkt` (diff=12, top=coordinate, first=`children[0]/y`)
  - `realworld/ptolemy/flattened/ptolemy_brewery_Brewery.elkg` (diff=15, top=section, first=`children[6]/y`)
  - `realworld/ptolemy/flattened/ptolemy_brewery_Brewery.elkt` (diff=15, top=section, first=`children[6]/y`)
- `children[*]/height`
  - `tests/layered/node_placement/flexible_ports/graph01.elkt` (diff=10, top=coordinate, first=`children[0]/height`)
  - `tickets/layered/297_sameSideInsideSelfLoop.elkt` (diff=12, top=section, first=`children[0]/height`)
  - `tests/layered/node_placement/flexible_ports/graph02.elkt` (diff=19, top=coordinate, first=`children[0]/height`)
- `edges[*]/labels[*]/y`
  - `tickets/layered/360_badSelfLoopLabelPlacement.elkt` (diff=9, top=section, first=`edges[1]/labels[0]/y`)
  - `tickets/layered/128_selfLoopLabelSpacing_complex.elkt` (diff=12, top=label, first=`edges[2]/labels[0]/y`)
- `children[*]/children[*]/children[*]/x`
  - `tickets/layered/665_includeChildrenDoesntStop.elkt` (diff=15, top=coordinate, first=`children[0]/children[0]/children[0]/x`)
- `children[*]/children[*]/ports[*]/x`
  - `tickets/layered/273_insideSelfLoopsWithLabels.elkt` (diff=16, top=coordinate, first=`children[0]/children[0]/ports[0]/x`)
- `children[*]/children[*]/ports[*]/y`
  - `tickets/layered/548_NPEWithInsideSelfLoops.elkt` (diff=12, top=coordinate, first=`children[0]/children[0]/ports[0]/y`)
