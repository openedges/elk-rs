use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{GraphAdapter, NodeAdapter};

use super::node_margin_calculator::NodeMarginCalculator;

pub struct NodeDimensionCalculation;

impl NodeDimensionCalculation {
    pub fn calculate_label_and_node_sizes<T, G>(adapter: &G)
    where
        G: GraphAdapter<T>,
    {
        // TODO: Port NodeLabelAndSizeCalculator; keep as no-op for now.
        let _ = adapter;
    }

    pub fn calculate_node_margins<T, G>(adapter: &G)
    where
        G: GraphAdapter<T>,
    {
        let mut calculator = NodeMarginCalculator::new(adapter);
        calculator.process();
    }

    pub fn get_node_margin_calculator<T, G>(adapter: &G) -> NodeMarginCalculator<'_, G, T>
    where
        G: GraphAdapter<T>,
    {
        NodeMarginCalculator::new(adapter)
    }

    pub fn sort_port_lists<T, G>(adapter: &G)
    where
        G: GraphAdapter<T>,
    {
        for node in adapter.get_nodes() {
            node.sort_port_list();
        }
    }
}
