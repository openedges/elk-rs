# Top Phase Focus Queue

- Top phase roots: p5_edge_routing, p4_node_placement
- Candidate models: **13**

## p5_edge_routing

- models=10, total_diffs=139, buckets={'high_20_cap': 6, 'low_1_5': 2, 'medium_6_19': 2}

| Priority | Bucket | Diffs | Model |
|---:|---|---:|---|
| 1 | low_1_5 | 2 | realworld/ptolemy/hierarchical/ptides_distributedpowerplant_DistributedPowerPlant.elkg |
| 2 | low_1_5 | 2 | realworld/ptolemy/hierarchical/ptides_distributedpowerplant_DistributedPowerPlant.elkt |
| 3 | medium_6_19 | 7 | tests/layered/self_loops/inside_outside.elkt |
| 4 | medium_6_19 | 8 | tickets/layered/368_selfLoopLabelsIOOBE.elkt |
| 5 | high_20_cap | 20 | realworld/ptolemy/flattened/ptides_distributedpowerplant_DistributedPowerPlant.elkg |
| 6 | high_20_cap | 20 | realworld/ptolemy/flattened/ptides_distributedpowerplant_DistributedPowerPlant.elkt |
| 7 | high_20_cap | 20 | realworld/ptolemy/flattened/ptides_powerplant_PowerPlant.elkg |
| 8 | high_20_cap | 20 | realworld/ptolemy/flattened/ptides_powerplant_PowerPlant.elkt |
| 9 | high_20_cap | 20 | realworld/ptolemy/hierarchical/ptides_powerplant_PowerPlant.elkg |
| 10 | high_20_cap | 20 | realworld/ptolemy/hierarchical/ptides_powerplant_PowerPlant.elkt |

## p4_node_placement

- models=3, total_diffs=31, buckets={'low_1_5': 1, 'medium_6_19': 1, 'high_20_cap': 1}

| Priority | Bucket | Diffs | Model |
|---:|---|---:|---|
| 1 | low_1_5 | 5 | tests/core/label_placement/port_labels/next_to_port_if_possible_inside.elkt |
| 2 | medium_6_19 | 6 | tests/layered/port_label_placement/multilabels_compound.elkt |
| 3 | high_20_cap | 20 | tickets/layered/213_componentsCompaction.elkt |

