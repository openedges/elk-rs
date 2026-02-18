use std::any::Any;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::service::{
    DiagramLayoutEngine, IDiagramLayoutConnector, ILayoutConfigurationStoreProvider,
    ILayoutListener, ILayoutSetup, LayoutConnectorsService, LayoutMapping,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

struct DummyConnector;

impl IDiagramLayoutConnector for DummyConnector {
    fn build_layout_graph(
        &self,
        _workbench_part: Option<&dyn Any>,
        _diagram_part: Option<&dyn Any>,
    ) -> Option<LayoutMapping> {
        let root = ElkGraphUtil::create_graph();
        let mut mapping = LayoutMapping::new(None);
        mapping.set_layout_graph(root);
        Some(mapping)
    }

    fn apply_layout(&self, _mapping: &mut LayoutMapping, _settings: &MapPropertyHolder) {}
}

struct DummySetup;

impl ILayoutSetup for DummySetup {
    fn supports(&self, _object: &dyn Any) -> bool {
        true
    }

    fn create_connector(&self) -> Box<dyn IDiagramLayoutConnector> {
        Box::new(DummyConnector)
    }

    fn configuration_store_provider(&self) -> Option<Box<dyn ILayoutConfigurationStoreProvider>> {
        None
    }
}

struct RecordingListener {
    about_calls: Arc<AtomicUsize>,
    done_calls: Arc<AtomicUsize>,
}

impl ILayoutListener for RecordingListener {
    fn layout_about_to_start(
        &self,
        _mapping: &LayoutMapping,
        _progress_monitor: &mut dyn org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor,
    ) {
        self.about_calls.fetch_add(1, Ordering::SeqCst);
    }

    fn layout_done(
        &self,
        _mapping: &LayoutMapping,
        _progress_monitor: &mut dyn org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor,
    ) {
        self.done_calls.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn layout_connectors_service_selects_connector() {
    let mut service = LayoutConnectorsService::new();
    service.register_setup(
        10,
        Box::new(DummySetup),
        Arc::new(|| Box::new(DummyConnector)),
        None,
    );

    let connector = service.get_connector(None, Some(&"diagram"));
    assert!(connector.is_some());
}

#[test]
fn layout_connectors_service_prefers_higher_priority() {
    let selected_priority = Arc::new(AtomicI32::new(-1));
    let mut service = LayoutConnectorsService::new();

    let low_priority = Arc::clone(&selected_priority);
    service.register_setup(
        5,
        Box::new(DummySetup),
        Arc::new(move || {
            low_priority.store(5, Ordering::SeqCst);
            Box::new(DummyConnector)
        }),
        None,
    );

    let high_priority = Arc::clone(&selected_priority);
    service.register_setup(
        20,
        Box::new(DummySetup),
        Arc::new(move || {
            high_priority.store(20, Ordering::SeqCst);
            Box::new(DummyConnector)
        }),
        None,
    );

    let _ = service.get_connector(None, Some(&"diagram"));
    assert_eq!(selected_priority.load(Ordering::SeqCst), 20);
}

#[test]
fn layout_connectors_service_notifies_listeners_and_removes() {
    let mut service = LayoutConnectorsService::new();
    let about_calls = Arc::new(AtomicUsize::new(0));
    let done_calls = Arc::new(AtomicUsize::new(0));
    let listener: Arc<dyn ILayoutListener> = Arc::new(RecordingListener {
        about_calls: Arc::clone(&about_calls),
        done_calls: Arc::clone(&done_calls),
    });

    service.add_layout_listener(listener.clone());
    let mapping = LayoutMapping::new(None);
    let mut monitor = NullElkProgressMonitor;

    service.fire_layout_about_to_start(&mapping, &mut monitor);
    service.fire_layout_done(&mapping, &mut monitor);

    assert_eq!(about_calls.load(Ordering::SeqCst), 1);
    assert_eq!(done_calls.load(Ordering::SeqCst), 1);

    service.remove_layout_listener(&listener);
    service.fire_layout_about_to_start(&mapping, &mut monitor);
    service.fire_layout_done(&mapping, &mut monitor);

    assert_eq!(about_calls.load(Ordering::SeqCst), 1);
    assert_eq!(done_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn diagram_layout_engine_invokes_connector() {
    let connector = DummyConnector;
    let mut engine = DiagramLayoutEngine::new();
    let mut monitor = NullElkProgressMonitor;
    let mapping = engine.run_with_progress(&connector, None, None, None, &mut monitor);
    assert!(mapping.is_some());
}
