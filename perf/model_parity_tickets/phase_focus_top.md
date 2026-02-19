# Top Phase Focus Queue

- Top phase roots: p4_node_placement, p5_edge_routing
- Candidate models: **10**

## p4_node_placement

- models=7, total_diffs=90, buckets={'medium_6_19': 4, 'high_20_cap': 2, 'low_1_5': 1}

| Priority | Bucket | Diffs | Model |
|---:|---|---:|---|
| 1 | low_1_5 | 4 | tickets/layered/425_selfLoopInCompoundNode.elkt |
| 2 | medium_6_19 | 7 | tickets/layered/502_collapsingCompoundNode.elkt |
| 3 | medium_6_19 | 8 | tickets/layered/182_minNodeSizeForHierarchicalNodes.elkt |
| 4 | medium_6_19 | 15 | tickets/layered/665_includeChildrenDoesntStop.elkt |
| 5 | medium_6_19 | 16 | tickets/layered/453_interactiveProblems.elkt |
| 6 | high_20_cap | 20 | tickets/layered/213_componentsCompaction.elkt |
| 7 | high_20_cap | 20 | tickets/layered/701_portLabels.elkt |

## p5_edge_routing

- models=3, total_diffs=34, buckets={'high_20_cap': 1, 'medium_6_19': 2}

| Priority | Bucket | Diffs | Model |
|---:|---|---:|---|
| 1 | medium_6_19 | 6 | tickets/layered/352_selfLoopNPEorAIOOBE.elkt |
| 2 | medium_6_19 | 8 | tickets/layered/368_selfLoopLabelsIOOBE.elkt |
| 3 | high_20_cap | 20 | tickets/layered/302_brokenSplineSelfLoops.elkt |

