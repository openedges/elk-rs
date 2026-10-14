use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::{
    elk_layered::ElkLayered,
    graph::transform::{ElkGraphTransformer, IGraphTransformer},
    graph::{LEdge, LGraph, LNode, LPort, Layer},
    intermediate::loops::{SelfLoopEdge, SelfLoopHolder, SelfLoopPort},
    options::InternalProperties,
};
use org_eclipse_elk_core::org::eclipse::elk::core::{
    graph_layout_engine::IGraphLayoutEngine,
    options::{CoreOptions, PortSide},
    recursive_graph_layout_engine::RecursiveGraphLayoutEngine,
    util::BasicProgressMonitor,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api;
use std::rc::Rc;
use std::sync::Arc;

fn pair_graph(parent_endpoint: bool) -> ElkNodeRef {
    let root = ElkGraphUtil::create_graph();
    let a = ElkGraphUtil::create_node(Some(root.clone()));
    let b = ElkGraphUtil::create_node(Some(root.clone()));
    for node in [&a, &b] {
        node.borrow_mut().connectable().shape().set_width(30.0);
        node.borrow_mut().connectable().shape().set_height(30.0);
    }
    ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(if parent_endpoint { root.clone() } else { a }),
        ElkConnectableShapeRef::Node(b),
    );
    root
}

#[test]
fn hierarchical_endpoints_do_not_keep_graph_alive() {
    for parent_endpoint in [false, true] {
        let root = pair_graph(parent_endpoint);
        let weak = Rc::downgrade(&root);
        drop(root);
        assert!(
            weak.upgrade().is_none(),
            "parent endpoint: {parent_endpoint}"
        );
    }
}

#[test]
fn layered_layout_releases_its_internal_graph() {
    layout_api::ensure_initialized();
    let root = pair_graph(false);
    let mut transformer = ElkGraphTransformer::new();
    let graph = transformer.import_graph(&root);
    let weak = Arc::downgrade(&graph);
    let mut engine = ElkLayered::new();
    let nodes: Vec<_> = graph
        .lock()
        .layerless_nodes()
        .iter()
        .map(Arc::downgrade)
        .collect();
    engine.do_layout(&graph, None);
    drop((transformer, root, graph));
    // The reusable engine must not retain the completed graph. Some processor
    // caches retain the last nodes until the engine itself is dropped.
    assert!(weak.upgrade().is_none());
    drop(engine);
    assert!(nodes.iter().all(|node| node.upgrade().is_none()));
}

#[test]
fn spacings_do_not_own_the_graph() {
    layout_api::ensure_initialized();
    use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::Spacings;
    let graph = LGraph::new();
    let weak = Arc::downgrade(&graph);
    let spacings = Spacings::new(&graph);
    graph
        .lock()
        .set_property(InternalProperties::SPACINGS, Some(spacings));
    drop(graph);
    assert!(weak.upgrade().is_none());
}

#[test]
fn self_loop_holder_does_not_own_its_node() {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    let port = LPort::new();
    LPort::set_node(&port, Some(node.clone()));
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(port.clone()));
    LEdge::set_target(&edge, Some(port.clone()));
    let holder = SelfLoopHolder::install(&node);
    let node_weak = Arc::downgrade(&node);
    let holder_weak = Arc::downgrade(&holder);
    drop((graph, node, port, edge, holder));
    assert!(node_weak.upgrade().is_none());
    assert!(holder_weak.upgrade().is_none());
}

#[test]
fn self_loop_edges_do_not_own_their_ports() {
    let port = LPort::new();
    let edge = LEdge::new();
    let sl_port = SelfLoopPort::new(&port);
    let sl_edge = SelfLoopEdge::new(&edge, &sl_port, &sl_port);
    let port_weak = Arc::downgrade(&sl_port);
    let edge_weak = Arc::downgrade(&sl_edge);
    drop((port, edge, sl_port, sl_edge));
    assert!(port_weak.upgrade().is_none());
    assert!(edge_weak.upgrade().is_none());
}

#[test]
fn spline_dependencies_do_not_own_their_segments() {
    use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p5edges::splines::spline_segment::{SplineSegment, Dependency, SideToProcess};
    let edge = LEdge::new();
    let a = SplineSegment::new_single_edge(&edge, SideToProcess::Left, SideToProcess::Right);
    let b = SplineSegment::new_single_edge(&edge, SideToProcess::Left, SideToProcess::Right);
    let dep = Dependency::new(&a, &b, 1);
    let aw = Arc::downgrade(&a);
    let bw = Arc::downgrade(&b);
    let dw = Arc::downgrade(&dep);
    drop((edge, a, b, dep));
    assert!(aw.upgrade().is_none());
    assert!(bw.upgrade().is_none());
    assert!(dw.upgrade().is_none());
}

#[test]
fn cached_crossing_minimizer_does_not_own_the_graph() {
    layout_api::ensure_initialized();
    use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph_configurator::GraphConfigurator;
    let graph = LGraph::new();
    let a = LNode::new(&graph);
    let b = LNode::new(&graph);
    let left = Layer::new(&graph);
    let right = Layer::new(&graph);
    graph
        .lock()
        .layers_mut()
        .extend([left.clone(), right.clone()]);
    LNode::set_layer(&a, Some(left.clone()));
    LNode::set_layer(&b, Some(right.clone()));
    let ap = LPort::new();
    let bp = LPort::new();
    LPort::set_node(&ap, Some(a.clone()));
    LPort::set_node(&bp, Some(b.clone()));
    ap.lock().set_side(PortSide::East);
    bp.lock().set_side(PortSide::West);
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(ap.clone()));
    LEdge::set_target(&edge, Some(bp.clone()));
    let mut configurator = GraphConfigurator::new();
    configurator.prepare_graph_for_layout(&graph);
    graph
        .lock()
        .set_property(InternalProperties::SPACINGS, None);
    let processors = graph
        .lock()
        .get_property(InternalProperties::PROCESSORS)
        .unwrap();
    let mut executed = 0;
    for processor in &processors {
        let mut processor = processor.lock();
        if processor
            .type_name()
            .ends_with("LayerSweepCrossingMinimizer")
        {
            processor.process(&mut graph.lock(), &mut BasicProgressMonitor::new());
            executed += 1;
        }
    }
    let weak = Arc::downgrade(&graph);
    drop((
        graph,
        a,
        b,
        left,
        right,
        ap,
        bp,
        edge,
        configurator,
        processors,
    ));
    assert!(executed > 0);
    assert!(weak.upgrade().is_none());
}

#[test]
fn rectpacking_releases_rows_for_all_whitespace_strategies() {
    layout_api::ensure_initialized();
    for equal in [false, true] {
        use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::util::rows_storage;
        use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::{
            options::RectPackingOptions, p3whitespaceelimination::WhiteSpaceEliminationStrategy,
        };
        let graph = pair_graph(false);
        graph
            .borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(
                CoreOptions::ALGORITHM,
                Some("org.eclipse.elk.rectpacking".to_owned()),
            );
        if equal {
            graph
                .borrow_mut()
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .set_property(
                    RectPackingOptions::WHITE_SPACE_ELIMINATION_STRATEGY,
                    Some(WhiteSpaceEliminationStrategy::EqualBetweenStructures),
                );
        }
        let child = graph.borrow_mut().children().get(0).unwrap();
        let weak = Rc::downgrade(&child);
        let root_weak = Rc::downgrade(&graph);
        let key = Rc::as_ptr(&graph) as usize;
        RecursiveGraphLayoutEngine::new().layout(&graph, &mut BasicProgressMonitor::new());
        drop((child, graph));
        assert!(root_weak.upgrade().is_none());
        assert!(weak.upgrade().is_none(), "equal whitespace: {equal}");
        assert!(rows_storage::get_rows(key).is_none());
    }
}

/// Stop only after temporary state exists, so cleanup is exercised rather than
/// returning before the allocation being tested.
struct InterruptMonitor {
    rows_key: Option<usize>,
    panic: bool,
    triggered: std::cell::Cell<bool>,
}

impl org_eclipse_elk_core::org::eclipse::elk::core::util::IElkCancelIndicator for InterruptMonitor {
    fn is_canceled(&self) -> bool {
        let ready = match self.rows_key {
            Some(key) => org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::util::rows_storage::get_rows(key).is_some(),
            None => org_eclipse_elk_core::org::eclipse::elk::core::layout_arena_context::with_layout_arena(|_| ()).is_some(),
        };
        if ready {
            self.triggered.set(true);
            assert!(!self.panic, "injected layout interruption");
        }
        ready
    }
}

impl org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor for InterruptMonitor {
    fn begin(&mut self, _: &str, _: f32) -> bool {
        true
    }
    fn worked(&mut self, _: f32) {}
    fn done(&mut self) {}
    fn is_running(&self) -> bool {
        true
    }
    fn task_name(&self) -> Option<&str> {
        None
    }
    fn sub_task(
        &mut self,
        _: f32,
    ) -> Box<dyn org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor> {
        Box::new(BasicProgressMonitor::new())
    }
}

#[test]
fn layout_arena_releases_graph_after_cancellation_and_panic() {
    use org_eclipse_elk_core::org::eclipse::elk::core::layout_arena_context::with_layout_arena;
    layout_api::ensure_initialized();
    for panic in [false, true] {
        let graph = pair_graph(false);
        let weak = Rc::downgrade(&graph);
        let mut monitor = InterruptMonitor {
            rows_key: None,
            panic,
            triggered: false.into(),
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            RecursiveGraphLayoutEngine::new().layout(&graph, &mut monitor);
        }));
        assert!(monitor.triggered.get());
        assert_eq!(result.is_err(), panic);
        assert!(with_layout_arena(|_| ()).is_none());
        drop(graph);
        assert!(weak.upgrade().is_none());
    }
}

#[test]
fn rectpacking_releases_rows_after_cancellation_and_panic() {
    use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::{
        rect_packing_layout_provider::RectPackingLayoutProvider, util::rows_storage,
    };
    layout_api::ensure_initialized();
    for panic in [false, true] {
        let graph = pair_graph(false);
        let key = Rc::as_ptr(&graph) as usize;
        let child = Rc::downgrade(&graph.borrow_mut().children().get(0).unwrap());
        let mut monitor = InterruptMonitor {
            rows_key: Some(key),
            panic,
            triggered: false.into(),
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            RectPackingLayoutProvider::new().layout(&graph, &mut monitor);
        }));
        assert!(monitor.triggered.get());
        assert_eq!(result.is_err(), panic);
        assert!(rows_storage::get_rows(key).is_none());
        drop(graph);
        assert!(child.upgrade().is_none());
    }
}

#[test]
fn nested_layout_arena_restores_outer_context_after_panic() {
    use org_eclipse_elk_core::org::eclipse::elk::core::layout_arena_context::{
        with_layout_arena, LayoutArenaGuard,
    };
    use org_eclipse_elk_graph::org::eclipse::elk::graph::ENodeId;
    let outer = pair_graph(false);
    let outer_guard = LayoutArenaGuard::new(&outer);
    let inner = pair_graph(false);
    let weak = Rc::downgrade(&inner);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _inner_guard = LayoutArenaGuard::new(&inner);
        assert!(with_layout_arena(|arena| Rc::ptr_eq(arena.node_ref(ENodeId(0)), &inner)).unwrap());
        panic!("injected nested layout failure");
    }));
    assert!(result.is_err());
    drop(inner);
    assert!(weak.upgrade().is_none());
    assert!(with_layout_arena(|arena| Rc::ptr_eq(arena.node_ref(ENodeId(0)), &outer)).unwrap());
    drop(outer_guard);
    assert!(with_layout_arena(|_| ()).is_none());
}

#[test]
fn layered_external_port_dummies_are_released() {
    use org_eclipse_elk_core::org::eclipse::elk::core::options::PortConstraints;
    layout_api::ensure_initialized();
    for side in [
        PortSide::West,
        PortSide::East,
        PortSide::North,
        PortSide::South,
    ] {
        let root = pair_graph(false);
        root.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(
                CoreOptions::PORT_CONSTRAINTS,
                Some(PortConstraints::FixedSide),
            );
        let port = ElkGraphUtil::create_port(Some(root.clone()));
        port.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_SIDE, Some(side));
        let child = root.borrow_mut().children().get(0).unwrap();
        ElkGraphUtil::create_simple_edge(
            ElkConnectableShapeRef::Port(port),
            ElkConnectableShapeRef::Node(child),
        );
        let mut transformer = ElkGraphTransformer::new();
        let graph = transformer.import_graph(&root);
        let nodes: Vec<_> = graph
            .lock()
            .layerless_nodes()
            .iter()
            .map(Arc::downgrade)
            .collect();
        let mut engine = ElkLayered::new();
        engine.do_layout(&graph, None);
        drop((transformer, root, graph, engine));
        assert!(
            nodes.iter().all(|node| node.upgrade().is_none()),
            "{side:?}"
        );
    }
}

#[test]
fn inside_self_loop_layout_releases_elk_graph() {
    layout_api::ensure_initialized();
    let root = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(root.clone()));
    let p = ElkGraphUtil::create_port(Some(node.clone()));
    let q = ElkGraphUtil::create_port(Some(node.clone()));
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE, Some(true));
    let edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(p.clone()),
        ElkConnectableShapeRef::Port(q.clone()),
    );
    edge.borrow_mut()
        .element()
        .properties_mut()
        .set_property(CoreOptions::INSIDE_SELF_LOOPS_YO, Some(true));
    let root_w = Rc::downgrade(&root);
    let node_w = Rc::downgrade(&node);
    let port_w = Rc::downgrade(&p);
    RecursiveGraphLayoutEngine::new().layout(&root, &mut BasicProgressMonitor::new());
    drop((root, node, p, q, edge));
    assert!(root_w.upgrade().is_none(), "root retained");
    assert!(node_w.upgrade().is_none(), "node retained");
    assert!(port_w.upgrade().is_none(), "port retained");
}

#[test]
fn detached_edge_skips_expired_endpoints() {
    use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkEdge, ElkNode};
    let edge = ElkEdge::new();
    let expired = ElkNode::new();
    let live = ElkNode::new();
    ElkEdge::add_source(&edge, ElkConnectableShapeRef::Node(expired.clone()));
    ElkEdge::add_source(&edge, ElkConnectableShapeRef::Node(live.clone()));
    drop(expired);
    let mut edge = edge.borrow_mut();
    assert_eq!(edge.sources().len(), 1);
    assert!(edge
        .sources()
        .get(0)
        .unwrap()
        .ptr_eq(&ElkConnectableShapeRef::Node(live.clone())));
    assert_eq!(edge.sources().iter().count(), 1);
    assert!(edge.sources().remove_at(0).is_some());
    assert!(edge.sources().is_empty());
    assert_eq!(live.borrow_mut().connectable().outgoing_edges().len(), 0);
}
