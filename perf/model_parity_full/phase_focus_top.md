# Top Phase Focus Queue

- Top phase roots: p4_node_placement, p5_edge_routing
- Candidate models: **4**

## p4_node_placement

- models=3, total_diffs=31, buckets={'low_1_5': 1, 'medium_6_19': 1, 'high_20_cap': 1}

| Priority | Bucket | Diffs | Model |
|---:|---|---:|---|
| 1 | low_1_5 | 5 | tests/core/label_placement/port_labels/next_to_port_if_possible_inside.elkt |
| 2 | medium_6_19 | 6 | tests/layered/port_label_placement/multilabels_compound.elkt |
| 3 | high_20_cap | 20 | tickets/layered/213_componentsCompaction.elkt |

## p5_edge_routing

- models=1, total_diffs=7, buckets={'medium_6_19': 1}

| Priority | Bucket | Diffs | Model |
|---:|---|---:|---|
| 1 | medium_6_19 | 7 | tests/layered/self_loops/inside_outside.elkt |

