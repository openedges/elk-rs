use std::collections::BTreeMap;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph, LNodeRef, LPort};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;

pub struct PartitionMidprocessor;

impl ILayoutProcessor<LGraph> for PartitionMidprocessor {
    fn process(&mut self, lgraph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Partition midprocessing", 1.0);

        let mut partition_to_nodes: BTreeMap<i32, Vec<LNodeRef>> = BTreeMap::new();
        for node in lgraph.layerless_nodes() {
            let partition = node
                .lock()
                .ok()
                .and_then(|mut node_guard| node_guard.get_property(CoreOptions::PARTITIONING_PARTITION));
            if let Some(partition) = partition {
                partition_to_nodes
                    .entry(partition)
                    .or_default()
                    .push(node.clone());
            }
        }

        let mut partition_ids: Vec<i32> = partition_to_nodes.keys().copied().collect();
        partition_ids.sort_unstable();

        let mut id_iter = partition_ids.into_iter();
        let Some(mut first_id) = id_iter.next() else {
            monitor.done();
            return;
        };

        for second_id in id_iter {
            let first_partition = partition_to_nodes
                .get(&first_id)
                .cloned()
                .unwrap_or_default();
            let second_partition = partition_to_nodes
                .get(&second_id)
                .cloned()
                .unwrap_or_default();
            connect_partitions(&first_partition, &second_partition);
            first_id = second_id;
        }

        monitor.done();
    }
}

fn connect_partitions(first_partition: &[LNodeRef], second_partition: &[LNodeRef]) {
    for node in first_partition {
        let source_port = LPort::new();
        if let Ok(mut source_guard) = source_port.lock() {
            source_guard.set_side(PortSide::East);
            source_guard.set_property(InternalProperties::PARTITION_DUMMY, Some(true));
        }
        LPort::set_node(&source_port, Some(node.clone()));

        for other_node in second_partition {
            let target_port = LPort::new();
            if let Ok(mut target_guard) = target_port.lock() {
                target_guard.set_side(PortSide::West);
                target_guard.set_property(InternalProperties::PARTITION_DUMMY, Some(true));
            }
            LPort::set_node(&target_port, Some(other_node.clone()));

            let edge = LEdge::new();
            if let Ok(mut edge_guard) = edge.lock() {
                edge_guard.set_property(InternalProperties::PARTITION_DUMMY, Some(true));
            }
            LEdge::set_source(&edge, Some(source_port.clone()));
            LEdge::set_target(&edge, Some(target_port));
        }
    }
}
