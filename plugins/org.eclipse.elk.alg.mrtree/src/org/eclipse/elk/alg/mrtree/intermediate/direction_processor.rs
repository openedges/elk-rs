use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::TGraphRef;
use crate::org::eclipse::elk::alg::mrtree::options::{InternalProperties, MrTreeOptions};

#[derive(Default)]
pub struct DirectionProcessor;

impl ILayoutProcessor<TGraphRef> for DirectionProcessor {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Process directions", 1.0);

        let direction = {
            let mut graph_guard = match graph.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    progress_monitor.done();
                    return;
                }
            };
            graph_guard
                .get_property(MrTreeOptions::DIRECTION)
                .unwrap_or(Direction::Undefined)
        };

        if direction != Direction::Down {
            let nodes = {
                let graph_guard = match graph.lock() {
                    Ok(guard) => guard,
                    Err(_) => {
                        progress_monitor.done();
                        return;
                    }
                };
                graph_guard.nodes().clone()
            };

            for node in nodes {
                if let Ok(mut node_guard) = node.lock() {
                    let mut x = node_guard
                        .get_property(InternalProperties::XCOOR)
                        .unwrap_or(0);
                    let mut y = node_guard
                        .get_property(InternalProperties::YCOOR)
                        .unwrap_or(0);
                    match direction {
                        Direction::Up => {
                            y *= -1;
                        }
                        Direction::Right => {
                            std::mem::swap(&mut x, &mut y);
                        }
                        Direction::Left => {
                            std::mem::swap(&mut x, &mut y);
                            x *= -1;
                        }
                        _ => {}
                    }
                    node_guard.set_property(InternalProperties::XCOOR, Some(x));
                    node_guard.set_property(InternalProperties::YCOOR, Some(y));
                }
            }
        }

        progress_monitor.done();
    }
}
