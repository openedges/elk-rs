use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Instant;

struct Allocator;
static LIVE: AtomicIsize = AtomicIsize::new(0);
static PEAK: AtomicIsize = AtomicIsize::new(0);
fn add(bytes: isize) {
    let now = LIVE.fetch_add(bytes, Ordering::Relaxed) + bytes;
    PEAK.fetch_max(now, Ordering::Relaxed);
}
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() {
            add(layout.size() as isize);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(layout);
        if !p.is_null() {
            add(layout.size() as isize);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as isize, Ordering::Relaxed);
        System.dealloc(p, layout);
    }
    unsafe fn realloc(&self, p: *mut u8, old: Layout, size: usize) -> *mut u8 {
        let next = System.realloc(p, old, size);
        if !next.is_null() {
            add(size as isize - old.size() as isize);
        }
        next
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

fn graph(count: usize, ordered: bool) -> String {
    let nodes = (0..count).map(|i| format!(
        r#"{{"id":"n{i}","width":40,"height":20,"x":{},"y":{},"ports":[{{"id":"p{i}a"}},{{"id":"p{i}b"}}],"labels":[{{"text":"node","width":24,"height":12}}]}}"#,
        (i % 32) * 50, (i / 32) * 30
    )).collect::<Vec<_>>().join(",");
    let edges = if ordered {
        (1..count)
            .map(|i| {
                format!(
                    r#"{{"id":"e{i}","sources":["n{}"],"targets":["n{i}"]}}"#,
                    (i - 1) / 2
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    } else {
        String::new()
    };
    format!(r#"{{"id":"root","children":[{nodes}],"edges":[{edges}]}}"#)
}

fn measure(name: &str, input: &str, options: &str, iterations: usize) {
    for _ in 0..3 {
        drop(layout_api::layout_json(input, options).unwrap());
    }
    let base = LIVE.load(Ordering::Relaxed);
    PEAK.store(base, Ordering::Relaxed);
    let start = Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(layout_api::layout_json(input, options).unwrap());
    }
    let elapsed = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
    let peak = PEAK.load(Ordering::Relaxed) - base;
    let retained = LIVE.load(Ordering::Relaxed) - base;
    println!("{name},{iterations},{elapsed:.6},{peak},{retained}");
}

fn main() {
    layout_api::ensure_initialized();
    println!("scenario,iterations,avg_ms,peak_live_delta_bytes,retained_bytes");
    let options = r#"{"elk.algorithm":"fixed","elk.direction":"RIGHT","elk.edgeRouting":"ORTHOGONAL","elk.hierarchyHandling":"SEPARATE_CHILDREN","elk.layered.considerModelOrder.strategy":"NODES_AND_EDGES","elk.layered.nodePlacement.strategy":"BRANDES_KOEPF","elk.layered.spacing.edgeEdgeBetweenLayers":"12","elk.layered.spacing.edgeNodeBetweenLayers":"20","elk.layered.spacing.nodeNodeBetweenLayers":"90","elk.spacing.componentComponent":"60","elk.spacing.nodeNode":"30"}"#;
    for count in [1024, 4096] {
        measure(
            &format!("global_defaults_{count}"),
            &graph(count, false),
            options,
            10,
        );
    }
    let options = r#"{"elk.algorithm":"layered","elk.randomSeed":1,"elk.layered.considerModelOrder.strategy":"NODES_AND_EDGES","elk.layered.nodePlacement.strategy":"NETWORK_SIMPLEX"}"#;
    for count in [128, 512] {
        measure(
            &format!("ordered_simplex_{count}"),
            &graph(count, true),
            options,
            10,
        );
    }
}
