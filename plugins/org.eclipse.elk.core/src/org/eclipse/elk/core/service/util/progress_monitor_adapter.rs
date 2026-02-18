use std::sync::Arc;

use crate::org::eclipse::elk::core::util::{
    BasicProgressMonitor, IElkCancelIndicator, IElkProgressMonitor, LoggedGraph, LoggedGraphType,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

pub trait IProgressMonitor: Send + Sync {
    fn begin_task(&self, name: &str, total_work: i32);
    fn sub_task(&self, name: &str) {
        let _ = name;
    }
    fn worked(&self, work: i32);
    fn done(&self);
    fn is_canceled(&self) -> bool;
}

pub struct ProgressMonitorAdapter {
    inner: Box<dyn IElkProgressMonitor>,
    progress_monitor: Arc<dyn IProgressMonitor>,
    submitted_work: i32,
    max_hierarchy_levels: i32,
    top_instance: bool,
}

impl ProgressMonitorAdapter {
    pub fn new(progress_monitor: Arc<dyn IProgressMonitor>) -> Self {
        ProgressMonitorAdapter {
            inner: Box::new(BasicProgressMonitor::new()),
            progress_monitor,
            submitted_work: 0,
            max_hierarchy_levels: -1,
            top_instance: true,
        }
    }

    pub fn with_max_hierarchy_levels(mut self, levels: i32) -> Self {
        self.max_hierarchy_levels = levels;
        self
    }

    fn should_report(&self) -> bool {
        self.max_hierarchy_levels != 0
    }
}

impl IElkCancelIndicator for ProgressMonitorAdapter {
    fn is_canceled(&self) -> bool {
        self.progress_monitor.is_canceled() || self.inner.is_canceled()
    }
}

impl IElkProgressMonitor for ProgressMonitorAdapter {
    fn begin(&mut self, name: &str, total_work: f32) -> bool {
        let started = self.inner.begin(name, total_work);
        if started && self.should_report() {
            if self.top_instance {
                let total = if total_work <= 0.0 {
                    -1
                } else {
                    total_work as i32
                };
                self.progress_monitor.begin_task(name, total);
            } else {
                self.progress_monitor.sub_task(name);
            }
        }
        started
    }

    fn worked(&mut self, work: f32) {
        self.inner.worked(work);
        if self.should_report() && self.top_instance {
            let delta = work as i32;
            if delta > 0 {
                self.submitted_work += delta;
                self.progress_monitor.worked(delta);
            }
        }
    }

    fn done(&mut self) {
        self.inner.done();
        if self.should_report() && self.top_instance {
            self.progress_monitor.done();
        }
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
        let child = ProgressMonitorAdapter {
            inner: self.inner.sub_task(work),
            progress_monitor: self.progress_monitor.clone(),
            submitted_work: 0,
            max_hierarchy_levels: next_levels,
            top_instance: false,
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
        self.inner.log(message);
    }

    fn logs(&self) -> Vec<String> {
        self.inner.logs()
    }

    fn log_graph(&mut self, graph: &ElkNodeRef, tag: &str) {
        self.inner.log_graph(graph, tag);
    }

    fn log_graph_typed(
        &mut self,
        graph: &dyn std::any::Any,
        tag: &str,
        graph_type: LoggedGraphType,
    ) {
        self.inner.log_graph_typed(graph, tag, graph_type);
    }

    fn logged_graphs(&self) -> Vec<LoggedGraph> {
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
