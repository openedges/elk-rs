# Top Phase Focus Queue

- Top phase roots: p4_node_placement, p5_edge_routing
- Candidate models: **8**

## p4_node_placement

- models=5, total_diffs=78, buckets={'high_20_cap': 2, 'medium_6_19': 3}

| Priority | Bucket | Diffs | Model |
|---:|---|---:|---|
| 1 | medium_6_19 | 7 | tickets/layered/502_collapsingCompoundNode.elkt |
| 2 | medium_6_19 | 15 | tickets/layered/665_includeChildrenDoesntStop.elkt |
| 3 | medium_6_19 | 16 | tickets/layered/453_interactiveProblems.elkt |
| 4 | high_20_cap | 20 | tickets/layered/213_componentsCompaction.elkt |
| 5 | high_20_cap | 20 | tickets/layered/701_portLabels.elkt |

## p5_edge_routing

- models=3, total_diffs=34, buckets={'high_20_cap': 1, 'medium_6_19': 2}

| Priority | Bucket | Diffs | Model |
|---:|---|---:|---|
| 1 | medium_6_19 | 6 | tickets/layered/352_selfLoopNPEorAIOOBE.elkt |
| 2 | medium_6_19 | 8 | tickets/layered/368_selfLoopLabelsIOOBE.elkt |
| 3 | high_20_cap | 20 | tickets/layered/302_brokenSplineSelfLoops.elkt |

