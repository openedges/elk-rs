use std::fmt;

use crate::org::eclipse::elk::alg::layered::graph::{LayerRef, LNodeRef, LPortRef};
use crate::org::eclipse::elk::alg::layered::options::Spacings;

use super::neighborhood_information::NeighborhoodInformation;
use super::util::{node_id, node_margin_bottom, node_margin_top, node_size_y, port_offset_y, port_node_id};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VDirection {
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HDirection {
    Right,
    Left,
}

pub struct BKAlignedLayout {
    pub(crate) root: Vec<usize>,
    pub(crate) block_size: Vec<f64>,
    pub(crate) align: Vec<usize>,
    pub(crate) inner_shift: Vec<f64>,
    pub(crate) sink: Vec<usize>,
    pub(crate) shift: Vec<f64>,
    pub(crate) y: Vec<Option<f64>>,
    pub(crate) vdir: Option<VDirection>,
    pub(crate) hdir: Option<HDirection>,
    pub(crate) su: Vec<bool>,
    pub(crate) od: Vec<bool>,
    pub(crate) layers: Vec<LayerRef>,
    pub(crate) nodes_by_id: Vec<LNodeRef>,
    pub(crate) spacings: Spacings,
}

impl BKAlignedLayout {
    pub fn new(
        layers: Vec<LayerRef>,
        nodes_by_id: Vec<LNodeRef>,
        spacings: Spacings,
        vdir: Option<VDirection>,
        hdir: Option<HDirection>,
    ) -> Self {
        let node_count = nodes_by_id.len();
        let mut root = Vec::with_capacity(node_count);
        let mut align = Vec::with_capacity(node_count);
        let mut sink = Vec::with_capacity(node_count);
        for i in 0..node_count {
            root.push(i);
            align.push(i);
            sink.push(i);
        }

        BKAlignedLayout {
            root,
            block_size: vec![0.0; node_count],
            align,
            inner_shift: vec![0.0; node_count],
            sink,
            shift: vec![0.0; node_count],
            y: vec![None; node_count],
            vdir,
            hdir,
            su: vec![false; node_count],
            od: vec![true; node_count],
            layers,
            nodes_by_id,
            spacings,
        }
    }

    pub fn cleanup(&mut self) {
        self.root.clear();
        self.block_size.clear();
        self.align.clear();
        self.inner_shift.clear();
        self.sink.clear();
        self.shift.clear();
        self.y.clear();
        self.su.clear();
        self.od.clear();
        self.layers.clear();
        self.nodes_by_id.clear();
    }

    pub fn layout_size(&self) -> f64 {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for layer in &self.layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let node_id = node_id(&node);
                let y_min = self.y[node_id].unwrap_or(0.0);
                let root_id = self.root[node_id];
                let y_max = y_min + self.block_size[root_id];
                min = min.min(y_min);
                max = max.max(y_max);
            }
        }

        max - min
    }

    pub fn calculate_delta(&self, src: &LPortRef, tgt: &LPortRef) -> f64 {
        let src_node_id = port_node_id(src);
        let tgt_node_id = port_node_id(tgt);

        let src_pos = self.y[src_node_id].unwrap_or(0.0)
            + self.inner_shift[src_node_id]
            + port_offset_y(src);
        let tgt_pos = self.y[tgt_node_id].unwrap_or(0.0)
            + self.inner_shift[tgt_node_id]
            + port_offset_y(tgt);
        tgt_pos - src_pos
    }

    pub fn shift_block(&mut self, block_node: usize, delta: f64) {
        let root = block_node;
        let mut current = root;
        let max_steps = self.align.len().max(1);
        let mut steps = 0usize;
        loop {
            let new_pos = self.y[current].unwrap_or(0.0) + delta;
            self.y[current] = Some(new_pos);
            current = self.align[current];
            if current == root || steps >= max_steps {
                if steps >= max_steps && std::env::var("ELK_TRACE_BK_GUARD").is_ok() {
                    eprintln!(
                        "bk-guard: shift_block loop hit max_steps root={} current={} max_steps={}",
                        root, current, max_steps
                    );
                }
                break;
            }
            steps += 1;
        }
    }

    pub fn check_space_above(
        &self,
        block_node: usize,
        delta: f64,
        ni: &NeighborhoodInformation,
    ) -> f64 {
        let mut available_space = delta;
        let root = block_node;
        let mut current = root;
        let max_steps = self.align.len().max(1);
        let mut steps = 0usize;

        loop {
            current = self.align[current];
            let min_y_current = self.min_y(current);
            if let Some(neighbor) = self.upper_neighbor(current, ni) {
                let max_y_neighbor = self.max_y(node_id(&neighbor));
                let spacing = self.spacings.get_vertical_spacing(&self.nodes_by_id[current], &neighbor);
                available_space = available_space.min(min_y_current - (max_y_neighbor + spacing));
            }
            if current == root {
                break;
            }
            steps += 1;
            if steps >= max_steps {
                if std::env::var("ELK_TRACE_BK_GUARD").is_ok() {
                    eprintln!(
                        "bk-guard: check_space_above loop hit max_steps root={} current={} max_steps={}",
                        root, current, max_steps
                    );
                }
                break;
            }
        }

        available_space
    }

    pub fn check_space_below(
        &self,
        block_node: usize,
        delta: f64,
        ni: &NeighborhoodInformation,
    ) -> f64 {
        let mut available_space = delta;
        let root = block_node;
        let mut current = root;
        let max_steps = self.align.len().max(1);
        let mut steps = 0usize;

        loop {
            current = self.align[current];
            let max_y_current = self.max_y(current);
            if let Some(neighbor) = self.lower_neighbor(current, ni) {
                let min_y_neighbor = self.min_y(node_id(&neighbor));
                let spacing = self.spacings.get_vertical_spacing(&self.nodes_by_id[current], &neighbor);
                available_space = available_space.min(min_y_neighbor - (max_y_current + spacing));
            }
            if current == root {
                break;
            }
            steps += 1;
            if steps >= max_steps {
                if std::env::var("ELK_TRACE_BK_GUARD").is_ok() {
                    eprintln!(
                        "bk-guard: check_space_below loop hit max_steps root={} current={} max_steps={}",
                        root, current, max_steps
                    );
                }
                break;
            }
        }

        available_space
    }

    pub fn min_y(&self, node_id: usize) -> f64 {
        let root_id = self.root[node_id];
        self.y[root_id].unwrap_or(0.0) + self.inner_shift[node_id] - node_margin_top(&self.nodes_by_id[node_id])
    }

    pub fn max_y(&self, node_id: usize) -> f64 {
        let root_id = self.root[node_id];
        self.y[root_id].unwrap_or(0.0)
            + self.inner_shift[node_id]
            + node_size_y(&self.nodes_by_id[node_id])
            + node_margin_bottom(&self.nodes_by_id[node_id])
    }

    fn upper_neighbor(&self, node_id: usize, ni: &NeighborhoodInformation) -> Option<LNodeRef> {
        let node = self.nodes_by_id.get(node_id)?.clone();
        let layer = node.lock().ok().and_then(|node_guard| node_guard.layer())?;
        let layer_nodes = layer
            .lock()
            .ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();
        let layer_index = *ni.node_index.get(node_id)?;
        if layer_index > 0 {
            return Some(layer_nodes[layer_index - 1].clone());
        }
        None
    }

    fn lower_neighbor(&self, node_id: usize, ni: &NeighborhoodInformation) -> Option<LNodeRef> {
        let node = self.nodes_by_id.get(node_id)?.clone();
        let layer = node.lock().ok().and_then(|node_guard| node_guard.layer())?;
        let layer_nodes = layer
            .lock()
            .ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();
        let layer_index = *ni.node_index.get(node_id)?;
        if layer_index + 1 < layer_nodes.len() {
            return Some(layer_nodes[layer_index + 1].clone());
        }
        None
    }
}

impl fmt::Display for BKAlignedLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result = String::new();
        if let Some(hdir) = self.hdir {
            match hdir {
                HDirection::Right => result.push_str("RIGHT"),
                HDirection::Left => result.push_str("LEFT"),
            }
        }
        if let Some(vdir) = self.vdir {
            match vdir {
                VDirection::Down => result.push_str("DOWN"),
                VDirection::Up => result.push_str("UP"),
            }
        } else {
            result.push_str("BALANCED");
        }
        write!(f, "{result}")
    }
}
