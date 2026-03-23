use std::collections::HashMap;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::{TGraphRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::options::InternalProperties;

#[derive(Default)]
pub struct FanProcessor {
    glo_fan_map: HashMap<String, i32>,
    glo_desc_map: HashMap<String, i32>,
}

impl ILayoutProcessor<TGraphRef> for FanProcessor {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Processor compute fanout", 1.0);

        self.glo_fan_map.clear();
        self.glo_desc_map.clear();

        let root = {
            let graph_guard = graph.lock();
            graph_guard
                .nodes()
                .iter()
                .find(|node| {
                    let node_guard = node.lock();
                    node_guard.get_property(InternalProperties::ROOT).unwrap_or(false)
                })
                .cloned()
        };

        if let Some(root) = root {
            self.calculate_fan(&[root]);
        }

        let nodes = {
            let graph_guard = graph.lock();
            graph_guard.nodes().clone()
        };

        for node in nodes {
            let mut node_guard = node.lock();
            let key = node_guard
                .get_property(InternalProperties::ID)
                .unwrap_or_default();
            let fan = self.glo_fan_map.get(&key).cloned().unwrap_or(0);
            node_guard.set_property(InternalProperties::FAN, Some(fan));
            let desc = self.glo_desc_map.get(&key).cloned().unwrap_or(0) + 1;
            node_guard.set_property(InternalProperties::DESCENDANTS, Some(desc));
        }

        progress_monitor.done();
    }
}

impl FanProcessor {
    fn calculate_fan(&mut self, current_level: &[TNodeRef]) {
        if current_level.is_empty() {
            return;
        }

        let mut next_level: Vec<TNodeRef> = Vec::new();
        let digits = ((current_level.len() as f64).log10().floor() as usize) + 1;
        let mut last_parent_id: Option<String> = None;
        let mut index: i32 = 0;
        let mut last_id = String::new();

        // Cache IDs during main loop to avoid re-locking in prefix computation
        let mut cached_ids: Vec<String> = Vec::with_capacity(current_level.len());

        for node in current_level {
            {
                let mut node_guard = node.lock();
                let parent_id = node_guard
                    .get_property(InternalProperties::ID)
                    .unwrap_or_default();
                if last_parent_id.as_deref() != Some(parent_id.as_str()) {
                    last_parent_id = Some(parent_id);
                    index = 0;
                }

                let id = if let Some(parent_id) = &last_parent_id {
                    if parent_id.is_empty() {
                        Self::format_right(index, digits)
                    } else {
                        format!("{}{}", parent_id, Self::format_right(index, digits))
                    }
                } else {
                    Self::format_right(index, digits)
                };
                index += 1;
                last_id = id.clone();
                cached_ids.push(id.clone());
                node_guard.set_property(InternalProperties::ID, Some(id));

                let children = node_guard.children_copy();
                for child in children {
                    {
                        let mut child_guard = child.lock();
                        child_guard.set_property(InternalProperties::ID, Some(last_id.clone()));
                    }
                    next_level.push(child);
                }
            }
        }

        // Prefix computation using cached IDs (zero locks)
        let mut local_fan_map: HashMap<String, i32> = HashMap::new();
        if digits > 0 && last_id.len() >= digits {
            for i in 0..(last_id.len().saturating_sub(digits)) {
                for id in &cached_ids {
                    let prefix: String = id.chars().take(i + 1).collect();
                    *local_fan_map.entry(prefix).or_insert(0) += 1;
                }
            }
        }

        for (key, value) in local_fan_map {
            let desc_value = self.glo_desc_map.get(&key).cloned().unwrap_or(0) + value;
            self.glo_desc_map.insert(key.clone(), desc_value);

            let fan_value = self.glo_fan_map.get(&key).cloned().unwrap_or(0);
            if fan_value < value {
                self.glo_fan_map.insert(key, value);
            }
        }

        self.calculate_fan(&next_level);
    }

    pub fn format_right(value: i32, len: usize) -> String {
        let mut s = value.to_string();
        while s.len() < len {
            s.insert(0, '0');
        }
        s
    }
}
