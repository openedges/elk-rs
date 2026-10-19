//! Pure Rust port of the [Eclipse Layout Kernel (ELK)](https://eclipse.dev/elk/).
//!
//! `elk-layout` is a facade over the `org-eclipse-elk-*` crates of the
//! [elk-rs](https://github.com/openedges/elk-rs) workspace, which mirror the ELK Java
//! plugins one to one. It re-exports them as modules and exposes the elkjs-compatible
//! JSON layout API at the top level, so one dependency is enough:
//!
//! ```
//! let graph = r#"{
//!   "id": "root",
//!   "layoutOptions": { "elk.algorithm": "layered" },
//!   "children": [
//!     { "id": "n1", "width": 30, "height": 30 },
//!     { "id": "n2", "width": 30, "height": 30 }
//!   ],
//!   "edges": [ { "id": "e1", "sources": ["n1"], "targets": ["n2"] } ]
//! }"#;
//!
//! // Global layout options (merged as defaults) go in the second argument.
//! let laid_out = elk_layout::layout_json(graph, "{}").unwrap();
//! assert!(laid_out.contains("\"x\""));
//! ```
//!
//! The same functions back the `@openedges/elk-layout` npm package (WASM and
//! Node.js N-API bindings). The full ELK object model and algorithm crates are
//! available through [`graph`], [`core`], [`graph_json`] and [`alg`].
#![forbid(unsafe_code)]

/// `org.eclipse.elk.graph`: the ElkGraph data model.
pub use org_eclipse_elk_graph as graph;
/// `org.eclipse.elk.core`: layout options, algorithm registry and the recursive layout engine.
pub use org_eclipse_elk_core as core;
/// `org.eclipse.elk.graph.json`: JSON import/export and the elkjs-compatible layout API.
pub use org_eclipse_elk_graph_json as graph_json;
/// `org.eclipse.elk.conn.gmf`: the GMF connector (Draw2D layouter).
pub use org_eclipse_elk_conn_gmf as conn_gmf;

/// The layout algorithm crates (`org.eclipse.elk.alg.*`).
pub mod alg {
    pub use org_eclipse_elk_alg_common as common;
    pub use org_eclipse_elk_alg_disco as disco;
    pub use org_eclipse_elk_alg_force as force;
    pub use org_eclipse_elk_alg_graphviz_dot as graphviz_dot;
    pub use org_eclipse_elk_alg_graphviz_layouter as graphviz_layouter;
    pub use org_eclipse_elk_alg_layered as layered;
    pub use org_eclipse_elk_alg_libavoid as libavoid;
    pub use org_eclipse_elk_alg_mrtree as mrtree;
    pub use org_eclipse_elk_alg_radial as radial;
    pub use org_eclipse_elk_alg_rectpacking as rectpacking;
    pub use org_eclipse_elk_alg_spore as spore;
    pub use org_eclipse_elk_alg_topdownpacking as topdownpacking;
    pub use org_eclipse_elk_alg_vertiflex as vertiflex;
}

/// The elkjs-compatible JSON API module (`org.eclipse.elk.graph.json.layout_api`).
pub use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api;
pub use layout_api::{
    ensure_initialized, known_layout_algorithms, known_layout_categories, known_layout_options,
    layout_json,
};
