# Top Phase Focus Queue

- Top phase roots: p4_node_placement, p5_edge_routing
- Candidate models: **13**

## p4_node_placement

- models=9, total_diffs=155, buckets={'low_1_5': 3, 'medium_6_19': 4, 'high_20_cap': 2}

| Priority | Bucket | Diffs | Model |
|---:|---|---:|---|
| 1 | low_1_5 | 1 | tickets/core/491_portSpacing.elkt |
| 2 | low_1_5 | 4 | tickets/core/562_insideSelfLoopAlgorithmResolving.elkt |
| 3 | low_1_5 | 4 | tickets/layered/425_selfLoopInCompoundNode.elkt |
| 4 | medium_6_19 | 7 | tickets/layered/502_collapsingCompoundNode.elkt |
| 5 | medium_6_19 | 8 | tickets/layered/182_minNodeSizeForHierarchicalNodes.elkt |
| 6 | medium_6_19 | 15 | tickets/layered/665_includeChildrenDoesntStop.elkt |
| 7 | medium_6_19 | 16 | tickets/layered/453_interactiveProblems.elkt |
| 8 | high_20_cap | 50 | tickets/layered/213_componentsCompaction.elkt |
| 9 | high_20_cap | 50 | tickets/layered/701_portLabels.elkt |

## p5_edge_routing

- models=4, total_diffs=59, buckets={'high_20_cap': 2, 'medium_6_19': 2}

| Priority | Bucket | Diffs | Model |
|---:|---|---:|---|
| 1 | medium_6_19 | 6 | tickets/layered/352_selfLoopNPEorAIOOBE.elkt |
| 2 | medium_6_19 | 8 | tickets/layered/368_selfLoopLabelsIOOBE.elkt |
| 3 | high_20_cap | 20 | tickets/layered/302_brokenSplineSelfLoops.elkt |
| 4 | high_20_cap | 25 | tickets/layered/371_strangeSplineSpacing.elkt |

