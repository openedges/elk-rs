use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;

use crate::org::eclipse::elk::core::util::{
    BasicProgressMonitor, IElkCancelIndicator, IElkProgressMonitor, NullElkProgressMonitor,
};

pub const MAX_PROGRESS_LEVELS: i32 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationStatus {
    Ok,
    Canceled,
    Error(String),
}

pub trait IMonitoredOperation {
    fn execute(&mut self, monitor: &mut dyn IElkProgressMonitor) -> OperationStatus;

    fn pre_exec(&mut self) {}

    fn post_exec(&mut self) {}
}

pub struct MonitoredOperation {
    canceled: Arc<AtomicBool>,
    cancel_indicator: Option<Arc<dyn IElkCancelIndicator + Send + Sync>>,
    timestamp: Option<Instant>,
}

impl MonitoredOperation {
    pub fn new(cancel_indicator: Option<Arc<dyn IElkCancelIndicator + Send + Sync>>) -> Self {
        MonitoredOperation {
            canceled: Arc::new(AtomicBool::new(false)),
            cancel_indicator,
            timestamp: None,
        }
    }

    pub fn cancel(&self) {
        self.canceled.store(true, Ordering::SeqCst);
    }

    pub fn is_canceled(&self) -> bool {
        self.canceled.load(Ordering::SeqCst)
            || self
                .cancel_indicator
                .as_ref()
                .map(|indicator| indicator.is_canceled())
                .unwrap_or(false)
    }

    pub fn timestamp(&self) -> Option<Instant> {
        self.timestamp
    }

    pub fn run_monitored<O: IMonitoredOperation>(&mut self, operation: &mut O) -> OperationStatus {
        self.timestamp = Some(Instant::now());
        if self.is_canceled() {
            return OperationStatus::Canceled;
        }

        operation.pre_exec();
        let base = BasicProgressMonitor::new();
        let mut monitor = CancelableProgressMonitor::new(
            Box::new(base),
            self.canceled.clone(),
            self.cancel_indicator.clone(),
        )
        .with_max_hierarchy_levels(MAX_PROGRESS_LEVELS);

        let result = operation.execute(&mut monitor);
        operation.post_exec();
        result
    }

    pub fn run_unmonitored<O: IMonitoredOperation>(
        &mut self,
        operation: &mut O,
    ) -> OperationStatus {
        self.timestamp = Some(Instant::now());
        if self.is_canceled() {
            return OperationStatus::Canceled;
        }
        operation.pre_exec();
        let mut monitor = NullElkProgressMonitor;
        let result = operation.execute(&mut monitor);
        operation.post_exec();
        result
    }
}

pub struct CancelableProgressMonitor {
    inner: Box<dyn IElkProgressMonitor>,
    canceled: Arc<AtomicBool>,
    cancel_indicator: Option<Arc<dyn IElkCancelIndicator + Send + Sync>>,
    max_hierarchy_levels: i32,
}

impl CancelableProgressMonitor {
    pub fn new(
        inner: Box<dyn IElkProgressMonitor>,
        canceled: Arc<AtomicBool>,
        cancel_indicator: Option<Arc<dyn IElkCancelIndicator + Send + Sync>>,
    ) -> Self {
        CancelableProgressMonitor {
            inner,
            canceled,
            cancel_indicator,
            max_hierarchy_levels: -1,
        }
    }

    pub fn with_max_hierarchy_levels(mut self, levels: i32) -> Self {
        self.max_hierarchy_levels = levels;
        self
    }
}

impl IElkCancelIndicator for CancelableProgressMonitor {
    fn is_canceled(&self) -> bool {
        if self.canceled.load(Ordering::SeqCst) {
            return true;
        }
        if let Some(indicator) = &self.cancel_indicator {
            if indicator.is_canceled() {
                return true;
            }
        }
        self.inner.is_canceled()
    }
}

impl IElkProgressMonitor for CancelableProgressMonitor {
    fn begin(&mut self, name: &str, total_work: f32) -> bool {
        self.inner.begin(name, total_work)
    }

    fn worked(&mut self, work: f32) {
        self.inner.worked(work)
    }

    fn done(&mut self) {
        self.inner.done()
    }

    fn is_running(&self) -> bool {
        self.inner.is_running()
    }

    fn task_name(&self) -> Option<&str> {
        self.inner.task_name()
    }

    fn sub_task(&mut self, work: f32) -> Box<dyn IElkProgressMonitor> {
        let next_levels = if self.max_hierarchy_levels > 0 {
            self.max_hierarchy_levels - 1
        } else {
            self.max_hierarchy_levels
        };
        let child = CancelableProgressMonitor {
            inner: self.inner.sub_task(work),
            canceled: self.canceled.clone(),
            cancel_indicator: self.cancel_indicator.clone(),
            max_hierarchy_levels: next_levels,
        };
        Box::new(child)
    }

    fn is_logging_enabled(&self) -> bool {
        self.inner.is_logging_enabled()
    }

    fn is_log_persistence_enabled(&self) -> bool {
        self.inner.is_log_persistence_enabled()
    }

    fn log(&mut self, message: &str) {
        self.inner.log(message)
    }

    fn logs(&self) -> Vec<String> {
        self.inner.logs()
    }

    fn log_graph(
        &mut self,
        graph: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef,
        tag: &str,
    ) {
        self.inner.log_graph(graph, tag)
    }

    fn log_graph_typed(
        &mut self,
        graph: &dyn std::any::Any,
        tag: &str,
        graph_type: crate::org::eclipse::elk::core::util::LoggedGraphType,
    ) {
        self.inner.log_graph_typed(graph, tag, graph_type)
    }

    fn logged_graphs(&self) -> Vec<crate::org::eclipse::elk::core::util::LoggedGraph> {
        self.inner.logged_graphs()
    }

    fn debug_folder(&self) -> Option<std::path::PathBuf> {
        self.inner.debug_folder()
    }

    fn is_execution_time_measured(&self) -> bool {
        self.inner.is_execution_time_measured()
    }

    fn execution_time(&self) -> f64 {
        self.inner.execution_time()
    }
}
