//! Centralized trace/debug flag configuration.
//!
//! All `ELK_TRACE_*` and `ELK_DEBUG_*` environment variables are read once at
//! startup and cached here. Access via `ElkTrace::global()`.
//!
//! **Not included here:**
//! - `ELK_TRACE_DIR`: read fresh per model by the parity trace infrastructure
//! - `ELK_TRACE_SIZING` in the graph crate: stays local (graph has no core dep)

use std::sync::LazyLock;

static INSTANCE: LazyLock<ElkTrace> = LazyLock::new(ElkTrace::from_env);

macro_rules! define_trace {
    (
        bool { $( $bool_field:ident : $bool_env:literal, )* }
        string { $( $str_field:ident : $str_env:literal, )* }
    ) => {
        pub struct ElkTrace {
            $( pub $bool_field: bool, )*
            $( pub $str_field: Option<String>, )*
        }

        impl ElkTrace {
            fn from_env() -> Self {
                ElkTrace {
                    $( $bool_field: std::env::var_os($bool_env).is_some(), )*
                    $( $str_field: std::env::var($str_env).ok(), )*
                }
            }
        }
    };
}

define_trace! {
    bool {
        // ── General ─────────────────────────────────────────────
        trace:                          "ELK_TRACE",
        phases:                         "ELK_TRACE_PHASES",
        phase_timing:                   "ELK_TRACE_PHASE_TIMING",
        processors:                     "ELK_TRACE_PROCESSORS",
        processor_timing:               "ELK_TRACE_PROCESSOR_TIMING",
        nodes:                          "ELK_TRACE_NODES",
        edge_wiring:                    "ELK_TRACE_EDGE_WIRING",
        resize:                         "ELK_TRACE_RESIZE",
        recursive_layout:               "ELK_TRACE_RECURSIVE_LAYOUT",

        // ── Sizing / labels ─────────────────────────────────────
        node_size:                      "ELK_TRACE_NODE_SIZE",
        sizing:                         "ELK_TRACE_SIZING",
        sizing_labels:                  "ELK_TRACE_SIZING_LABELS",

        // ── Crossing minimization ───────────────────────────────
        crossmin:                       "ELK_TRACE_CROSSMIN",
        crossmin_timing:                "ELK_TRACE_CROSSMIN_TIMING",
        crossmin_stats:                 "ELK_TRACE_CROSSMIN_STATS",
        crossmin_constraints:           "ELK_TRACE_CROSSMIN_CONSTRAINTS",
        crossings_breakdown:            "ELK_TRACE_CROSSINGS_BREAKDOWN",
        greedy_switch:                  "ELK_TRACE_GREEDY_SWITCH",
        greedy_ports:                   "ELK_TRACE_GREEDY_PORTS",
        port_ranks:                     "ELK_TRACE_PORT_RANKS",
        forster_groups:                 "ELK_TRACE_FORSTER_GROUPS",

        // ── Cycle breaking ──────────────────────────────────────
        cycle_choices:                  "ELK_TRACE_CYCLE_CHOICES",
        cycle_reversals:                "ELK_TRACE_CYCLE_REVERSALS",

        // ── BK node placer ──────────────────────────────────────
        bk:                             "ELK_TRACE_BK",
        bk_align:                       "ELK_TRACE_BK_ALIGN",
        bk_classes:                     "ELK_TRACE_BK_CLASSES",
        bk_conflicts:                   "ELK_TRACE_BK_CONFLICTS",
        bk_guard:                       "ELK_TRACE_BK_GUARD",
        bk_inner:                       "ELK_TRACE_BK_INNER",
        bk_layouts:                     "ELK_TRACE_BK_LAYOUTS",
        bk_place_block:                 "ELK_TRACE_BK_PLACE_BLOCK",
        bk_node_state:                  "ELK_TRACE_BK_NODE_STATE",
        bk_thresh:                      "ELK_TRACE_BK_THRESH",

        // ── Network simplex ─────────────────────────────────────
        network_simplex:                "ELK_TRACE_NETWORK_SIMPLEX",

        // ── Edge routing ────────────────────────────────────────
        ortho:                          "ELK_TRACE_ORTHO",
        compound_width:                 "ELK_TRACE_COMPOUND_WIDTH",
        edge_apply:                     "ELK_TRACE_EDGE_APPLY",
        edge_offsets:                   "ELK_TRACE_EDGE_OFFSETS",
        edge_origin_map:                "ELK_TRACE_EDGE_ORIGIN_MAP",

        // ── Compound / hierarchy ────────────────────────────────
        inside_yo:                      "ELK_TRACE_INSIDE_YO",
        external_ports:                 "ELK_TRACE_EXTERNAL_PORTS",
        hn:                             "ELK_TRACE_HN",
        hier_port_ortho:                "ELK_TRACE_HIER_PORT_ORTHO",

        // ── Intermediate processors ─────────────────────────────
        ilc:                            "ELK_TRACE_ILC",
        ns:                             "ELK_TRACE_NS",
        long_edge_split:                "ELK_TRACE_LONG_EDGE_SPLIT",
        label_dummy_switcher:           "ELK_TRACE_LABEL_DUMMY_SWITCHER",
        layer_height:                   "ELK_TRACE_LAYER_HEIGHT",
        core_port_sort:                 "ELK_TRACE_CORE_PORT_SORT",

        // ── Import ──────────────────────────────────────────────
        import_edge_selection:          "ELK_TRACE_IMPORT_EDGE_SELECTION",
        import_port_order:              "ELK_TRACE_IMPORT_PORT_ORDER",
        json_edge_adjust:               "ELK_TRACE_JSON_EDGE_ADJUST",

        // ── Force / Stress ──────────────────────────────────────
        stress:                         "ELK_TRACE_STRESS",

        // ── Debug flags ─────────────────────────────────────────
        debug_edges:                    "ELK_DEBUG_EDGES",
    }
    string {
        nodes_filter:                   "ELK_TRACE_NODES_FILTER",
        barycenter_layer_pattern:       "ELK_TRACE_BARYCENTER_LAYER_PATTERN",
        bk_node_filter:                 "ELK_TRACE_BK_NODE_FILTER",
        debug_crossmin_force_sweep:     "ELK_DEBUG_CROSSMIN_FORCE_SWEEP",
        debug_cycle_force_reverse_origins: "ELK_DEBUG_CYCLE_FORCE_REVERSE_ORIGINS",
        debug_cycle_random_prefetch:    "ELK_DEBUG_CYCLE_RANDOM_PREFETCH",
    }
}

impl ElkTrace {
    #[inline]
    pub fn global() -> &'static ElkTrace {
        &INSTANCE
    }
}
