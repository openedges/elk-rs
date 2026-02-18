use std::sync::Arc;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::NodeMicroLayout;
use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::algorithm_assembler::{
    AlgorithmAssembler, SharedProcessor,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::unsupported_configuration::UnsupportedConfigurationException;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::vertiflex::edge_routing_strategy::EdgeRoutingStrategy;
use crate::org::eclipse::elk::alg::vertiflex::internal_properties::InternalProperties;
use crate::org::eclipse::elk::alg::vertiflex::options::VertiFlexOptions;
use crate::org::eclipse::elk::alg::vertiflex::p1yplacement::NodeYPlacerStrategy;
use crate::org::eclipse::elk::alg::vertiflex::p2relative::RelativeXPlacerStrategy;
use crate::org::eclipse::elk::alg::vertiflex::p3absolute::AbsoluteXPlacerStrategy;
use crate::org::eclipse::elk::alg::vertiflex::p4edgerouting::EdgerouterStrategy;
use crate::org::eclipse::elk::alg::vertiflex::vertiflex_layout_phases::VertiFlexLayoutPhases;
use crate::org::eclipse::elk::alg::vertiflex::vertiflex_util::VertiFlexUtil;

pub struct VertiFlexLayoutProvider;

impl VertiFlexLayoutProvider {
    pub fn new() -> Self {
        VertiFlexLayoutProvider
    }

    pub fn assemble_algorithm(graph: &ElkNodeRef) -> Vec<SharedProcessor<ElkNodeRef>> {
        let mut algorithm_assembler: AlgorithmAssembler<VertiFlexLayoutPhases, ElkNodeRef> =
            AlgorithmAssembler::create();

        let node_y_factory: Arc<dyn ILayoutPhaseFactory<VertiFlexLayoutPhases, ElkNodeRef>> =
            Arc::new(NodeYPlacerStrategy::SimpleYPlacing);
        let relative_factory: Arc<dyn ILayoutPhaseFactory<VertiFlexLayoutPhases, ElkNodeRef>> =
            Arc::new(RelativeXPlacerStrategy::SimpleXPlacing);
        let absolute_factory: Arc<dyn ILayoutPhaseFactory<VertiFlexLayoutPhases, ElkNodeRef>> =
            Arc::new(AbsoluteXPlacerStrategy::AbsoluteXPlacing);

        let routing_strategy =
            match node_get_property(graph, VertiFlexOptions::LAYOUT_STRATEGY).unwrap_or_default() {
                EdgeRoutingStrategy::Bend => EdgerouterStrategy::BendRouting,
                EdgeRoutingStrategy::Straight => EdgerouterStrategy::DirectRouting,
            };
        let edge_factory: Arc<dyn ILayoutPhaseFactory<VertiFlexLayoutPhases, ElkNodeRef>> =
            Arc::new(routing_strategy);

        algorithm_assembler.set_phase(VertiFlexLayoutPhases::P1NodeYPlacement, node_y_factory);
        algorithm_assembler.set_phase(
            VertiFlexLayoutPhases::P2NodeRelativePlacement,
            relative_factory,
        );
        algorithm_assembler.set_phase(
            VertiFlexLayoutPhases::P3NodeAbsolutePlacement,
            absolute_factory,
        );
        algorithm_assembler.set_phase(VertiFlexLayoutPhases::P4EdgeRouting, edge_factory);

        algorithm_assembler.build(graph)
    }

    fn check_vertical_constraint_validity(
        root: &ElkNodeRef,
        current_min_constraint: f64,
        node_node_spacing: f64,
    ) {
        let mut root_height = current_min_constraint;
        if node_has_property(root, VertiFlexOptions::VERTICAL_CONSTRAINT) {
            if let Some(value) = node_get_property(root, VertiFlexOptions::VERTICAL_CONSTRAINT) {
                root_height = value;
            }
        }

        let margins = node_get_property(root, CoreOptions::MARGINS).unwrap_or_default();
        let new_min_constraint =
            root_height + node_height(root) + margins.bottom.max(node_node_spacing);

        for edge in ElkGraphUtil::all_outgoing_edges(root) {
            let Some(child) = edge_target_node(&edge) else {
                continue;
            };
            if node_has_property(&child, VertiFlexOptions::VERTICAL_CONSTRAINT) {
                let child_constraint =
                    node_get_property(&child, VertiFlexOptions::VERTICAL_CONSTRAINT).unwrap_or(0.0);
                let child_margin =
                    node_get_property(&child, CoreOptions::MARGINS).unwrap_or_default();
                if new_min_constraint > child_constraint + child_margin.top {
                    let identifier = node_identifier(&child);
                    let message = format!(
                        "Invalid vertical constraints. Node {} has a vertical constraint that is too low for its ancestors.",
                        identifier
                    );
                    panic!("{}", UnsupportedConfigurationException::new(message));
                }
            }
        }

        for edge in ElkGraphUtil::all_outgoing_edges(root) {
            let Some(child) = edge_target_node(&edge) else {
                continue;
            };
            Self::check_vertical_constraint_validity(&child, new_min_constraint, node_node_spacing);
        }
    }

    fn set_graph_size(&self, graph: &ElkNodeRef) {
        let padding = node_get_property(graph, CoreOptions::PADDING).unwrap_or_default();

        let children: Vec<ElkNodeRef> = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.children().iter().cloned().collect()
        };
        let mut max_x: f64 = 0.0;
        let mut max_y: f64 = 0.0;
        for node in children {
            let margins = node_get_property(&node, CoreOptions::MARGINS).unwrap_or_default();
            let x = node_x(&node);
            let y = node_y(&node);
            let width = node_width(&node);
            let height = node_height(&node);

            max_x = max_x.max(x + width + margins.right);
            max_y = max_y.max(y + height + margins.bottom);
        }

        let mut graph_mut = graph.borrow_mut();
        let shape = graph_mut.connectable().shape();
        shape.set_width(max_x + padding.right);
        shape.set_height(max_y + padding.bottom);
    }
}

impl Default for VertiFlexLayoutProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphLayoutEngine for VertiFlexLayoutProvider {
    fn layout(
        &mut self,
        layout_graph: &ElkNodeRef,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        let algorithm = Self::assemble_algorithm(layout_graph);
        progress_monitor.begin("Tree layout", algorithm.len() as f32);

        let node_node_spacing =
            node_get_property(layout_graph, CoreOptions::SPACING_NODE_NODE).unwrap_or(0.0);

        let omit_micro = node_get_property(layout_graph, VertiFlexOptions::OMIT_NODE_MICRO_LAYOUT)
            .unwrap_or(false);
        if !omit_micro {
            NodeMicroLayout::for_graph(layout_graph.clone()).execute();
        }

        let Some(root) = VertiFlexUtil::find_root(layout_graph) else {
            panic!(
                "{}",
                UnsupportedConfigurationException::new("The given graph is not a tree!")
            );
        };

        let children: Vec<ElkNodeRef> = {
            let mut graph_mut = layout_graph.borrow_mut();
            graph_mut.children().iter().cloned().collect()
        };
        for child in &children {
            let number_of_parents = ElkGraphUtil::all_incoming_edges(child).len();
            if number_of_parents > 1 {
                panic!(
                    "{}",
                    UnsupportedConfigurationException::new(
                        "The given graph is not an acyclic tree!"
                    )
                );
            }
            let mut child_mut = child.borrow_mut();
            let shape = child_mut.connectable().shape();
            shape.set_x(0.0);
            shape.set_y(0.0);
        }

        Self::check_vertical_constraint_validity(&root, 0.0, node_node_spacing);

        for (index, node) in children.iter().enumerate() {
            node_set_property(
                node,
                InternalProperties::NODE_MODEL_ORDER,
                Some(index as i32),
            );
        }

        let mut graph_ref = layout_graph.clone();
        for processor in &algorithm {
            let mut processor_guard = processor.lock().expect("processor lock");
            let mut sub = progress_monitor.sub_task(1.0);
            processor_guard.process(&mut graph_ref, sub.as_mut());
        }

        self.set_graph_size(layout_graph);

        progress_monitor.done();
    }
}

impl AbstractLayoutProvider for VertiFlexLayoutProvider {}

fn node_get_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
) -> Option<T> {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    props.get_property(property)
}

fn node_set_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: Option<T>,
) {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    props.set_property(property, value);
}

fn node_has_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
) -> bool {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    props.has_property(property)
}

fn node_x(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().x()
}

fn node_y(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().y()
}

fn node_width(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().width()
}

fn node_height(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().height()
}

fn edge_target_node(
    edge: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeRef,
) -> Option<ElkNodeRef> {
    let edge_borrow = edge.borrow();
    let target = edge_borrow.targets_ro().get(0)?;
    drop(edge_borrow);
    ElkGraphUtil::connectable_shape_to_node(&target)
}

fn node_identifier(node: &ElkNodeRef) -> String {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .identifier()
        .unwrap_or("<unknown>")
        .to_string()
}
