//! Measure live allocations, not RSS (which includes allocator-retained free pages).
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api;
use serde_json::json;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};

struct CountingAllocator;
static LIVE_BYTES: AtomicIsize = AtomicIsize::new(0);
static LIVE_ALLOCS: AtomicIsize = AtomicIsize::new(0);
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() {
            LIVE_BYTES.fetch_add(layout.size() as isize, Ordering::Relaxed);
            LIVE_ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        LIVE_BYTES.fetch_sub(layout.size() as isize, Ordering::Relaxed);
        LIVE_ALLOCS.fetch_sub(1, Ordering::Relaxed);
        System.dealloc(p, layout);
    }
    unsafe fn realloc(&self, p: *mut u8, old: Layout, new_size: usize) -> *mut u8 {
        let next = System.realloc(p, old, new_size);
        if !next.is_null() {
            LIVE_BYTES.fetch_add(new_size as isize - old.size() as isize, Ordering::Relaxed);
        }
        next
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn check_repeated_layout(graph: &str) {
    check_layout_allocations(graph, 100);
}

fn check_reused_layered_engine(graph: &str) {
    use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::{
        elk_layered::ElkLayered,
        graph::transform::{ElkGraphTransformer, IGraphTransformer},
    };
    use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::ElkGraphJson;

    let before = (
        LIVE_BYTES.load(Ordering::Relaxed),
        LIVE_ALLOCS.load(Ordering::Relaxed),
    );
    let mut engine = ElkLayered::new();
    let mut run = || {
        let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
        let mut transformer = ElkGraphTransformer::new();
        let internal = transformer.import_graph(&root);
        engine.do_layout(&internal, None);
    };
    for _ in 0..10 {
        run();
    }
    // A reusable engine may cache its last graph's nodes, but cannot grow with
    // the number of completed requests. Dropping it must release those caches.
    for _ in 0..3 {
        let bytes = LIVE_BYTES.load(Ordering::Relaxed);
        let allocations = LIVE_ALLOCS.load(Ordering::Relaxed);
        for _ in 0..100 {
            run();
        }
        assert_eq!(
            (
                LIVE_BYTES.load(Ordering::Relaxed) - bytes,
                LIVE_ALLOCS.load(Ordering::Relaxed) - allocations
            ),
            (0, 0),
            "reused engine: {graph}"
        );
    }
    drop(engine);
    assert_eq!(
        (
            LIVE_BYTES.load(Ordering::Relaxed),
            LIVE_ALLOCS.load(Ordering::Relaxed)
        ),
        before,
        "engine caches after drop: {graph}"
    );
}

fn check_layout_allocations(graph: &str, iterations: usize) {
    for _ in 0..10 {
        drop(layout_api::layout_json(graph, "{}").unwrap());
    }
    for _ in 0..3 {
        let bytes = LIVE_BYTES.load(Ordering::Relaxed);
        let allocations = LIVE_ALLOCS.load(Ordering::Relaxed);
        for _ in 0..iterations {
            drop(layout_api::layout_json(graph, "{}").unwrap());
        }
        let remaining_bytes = LIVE_BYTES.load(Ordering::Relaxed) - bytes;
        let remaining_allocations = LIVE_ALLOCS.load(Ordering::Relaxed) - allocations;
        assert_eq!((remaining_bytes, remaining_allocations), (0, 0), "{graph}");
    }
}

#[test]
fn repeated_layouts_release_all_request_allocations() {
    layout_api::ensure_initialized();
    check_structured_graphs();
    // Optional full-corpus regression using Java-exported model inputs.
    // Kept in the same test so measurements never overlap another test.
    if let Ok(path) = std::env::var("ELK_MEMORY_MANIFEST") {
        // The parity scripts write paths relative to the workspace, whereas
        // Cargo runs this test with the package directory as its working directory.
        let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let manifest = std::fs::read_to_string(workspace.join(path)).unwrap();
        for row in manifest.lines().skip(1) {
            let input = row.split('\t').nth(1).expect("input_json column");
            let graph = std::fs::read_to_string(workspace.join(input))
                .unwrap_or_else(|error| panic!("cannot read {input}: {error}"));
            check_layout_allocations(&graph, 2);
        }
    }
    for algorithm in [
        "layered",
        "force",
        "stress",
        "mrtree",
        "radial",
        "rectpacking",
        "sporeOverlap",
        "sporeCompaction",
        "vertiflex",
    ] {
        for (routing, target) in [("ORTHOGONAL", "b"), ("SPLINES", "b"), ("ORTHOGONAL", "a")] {
            if algorithm != "layered" && (routing == "SPLINES" || target == "a") {
                continue;
            }
            let graph = format!(
                r#"{{"id":"root","layoutOptions":{{"elk.algorithm":"{algorithm}","elk.edgeRouting":"{routing}"}},"children":[{{"id":"a","width":30,"height":30}},{{"id":"b","width":30,"height":30}}],"edges":[{{"id":"e","sources":["a"],"targets":["{target}"]}}]}}"#
            );
            check_repeated_layout(&graph);
            if algorithm == "layered" {
                check_reused_layered_engine(&graph);
            }
        }
    }
}

// Keep all allocation checks in the single test above: the global allocator
// must not observe another test running concurrently.
fn check_structured_graphs() {
    check_repeated_layout(&json!({"id":"root", "children":[{"id":"node", "ports":[{"id":"p"},{"id":"q"}],
        "layoutOptions":{"elk.insideSelfLoops.activate":true}}], "edges":[{"id":"loop","sources":["p"],"targets":["q"],
        "layoutOptions":{"elk.insideSelfLoops.yo":true}}]}).to_string());

    let graph = json!({"id":"root", "layoutOptions":{"elk.algorithm":"layered"},
        "children":[{"id":"a", "width":30, "height":30}, {"id":"b", "width":30, "height":30}],
        "edges":[{"id":"e", "sources":["a"], "targets":["b"]}]});
    for placement in ["HEAD", "TAIL", "CENTER"] {
        let mut labeled = graph.clone();
        labeled["edges"][0]["labels"] = json!([{"id":"label", "text":"edge label", "width":40, "height":12,
            "layoutOptions":{"elk.edgeLabels.placement":placement}}]);
        check_repeated_layout(&labeled.to_string());
    }
    for side in ["NORTH", "SOUTH"] {
        for routing in ["ORTHOGONAL", "POLYLINE", "SPLINES"] {
            let mut ports = graph.clone();
            ports["layoutOptions"]["elk.edgeRouting"] = json!(routing);
            ports["children"][0]["ports"] =
                json!([{"id":"p", "width":5,"height":5, "layoutOptions":{"elk.port.side":side}}]);
            ports["children"][0]["layoutOptions"] = json!({"elk.portConstraints":"FIXED_SIDE"});
            ports["edges"][0]["sources"] = json!(["p"]);
            check_repeated_layout(&ports.to_string());
        }
    }
    let hierarchy = json!({"id":"root", "layoutOptions":{"elk.algorithm":"layered","elk.hierarchyHandling":"INCLUDE_CHILDREN"},
        "children":[{"id":"parent","children":[{"id":"a","width":30,"height":30}]},{"id":"b","width":30,"height":30}],
        "edges":[{"id":"e","sources":["a"],"targets":["b"]}]});
    check_repeated_layout(&hierarchy.to_string());
    for placement in ["HEAD", "TAIL", "CENTER"] {
        let mut labeled = hierarchy.clone();
        labeled["edges"][0]["labels"] = json!([{"id":"label", "text":"hierarchical edge", "width":40, "height":12,
            "layoutOptions":{"elk.edgeLabels.placement":placement}}]);
        check_repeated_layout(&labeled.to_string());
    }
    for algorithm in ["layered", "mrtree", "radial", "rectpacking"] {
        let children: Vec<_> = (0..8)
            .map(|i| json!({"id":format!("n{i}"),"width":30,"height":30}))
            .collect();
        let edges: Vec<_> = (1..8)
            .map(|i| json!({"id":format!("e{i}"),"sources":["n0"],"targets":[format!("n{i}")]}))
            .collect();
        check_repeated_layout(&json!({"id":"root", "layoutOptions":{"elk.algorithm":algorithm}, "children":children, "edges":edges}).to_string());
    }
}
