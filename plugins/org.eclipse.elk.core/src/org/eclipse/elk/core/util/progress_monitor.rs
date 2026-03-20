use std::path::PathBuf;
use std::time::Instant;

use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::core::util::ElkUtil;

pub trait IElkCancelIndicator {
    fn is_canceled(&self) -> bool;
}

pub const UNKNOWN_WORK: f32 = -1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoggedGraphType {
    Elk,
    Json,
    Dot,
    Svg,
}

#[derive(Clone, Debug)]
pub struct LoggedGraph {
    graph_type: LoggedGraphType,
    tag: String,
}

impl LoggedGraph {
    pub fn new(tag: impl Into<String>, graph_type: LoggedGraphType) -> Self {
        LoggedGraph {
            graph_type,
            tag: tag.into(),
        }
    }

    pub fn graph_type(&self) -> LoggedGraphType {
        self.graph_type
    }

    pub fn tag(&self) -> &str {
        &self.tag
    }
}

pub trait IElkProgressMonitor: IElkCancelIndicator {
    fn begin(&mut self, name: &str, total_work: f32) -> bool;
    fn worked(&mut self, work: f32);
    fn done(&mut self);
    fn is_running(&self) -> bool;
    fn task_name(&self) -> Option<&str>;
    fn sub_task(&mut self, work: f32) -> Box<dyn IElkProgressMonitor>;

    fn sub_monitors(&self) -> Vec<&dyn IElkProgressMonitor> {
        Vec::new()
    }

    fn parent_monitor(&self) -> Option<&dyn IElkProgressMonitor> {
        None
    }

    fn is_logging_enabled(&self) -> bool {
        false
    }

    fn is_log_persistence_enabled(&self) -> bool {
        false
    }

    fn log(&mut self, _message: &str) {}

    fn logs(&self) -> Vec<String> {
        Vec::new()
    }

    fn log_graph(&mut self, _graph: &ElkNodeRef, _tag: &str) {}

    fn log_graph_typed(
        &mut self,
        _graph: &dyn std::any::Any,
        _tag: &str,
        _graph_type: LoggedGraphType,
    ) {
    }

    fn logged_graphs(&self) -> Vec<LoggedGraph> {
        Vec::new()
    }

    fn debug_folder(&self) -> Option<PathBuf> {
        None
    }

    fn is_execution_time_measured(&self) -> bool {
        false
    }

    fn execution_time(&self) -> f64 {
        0.0
    }
}

#[derive(Clone, Default)]
pub struct NullElkProgressMonitor;

impl IElkCancelIndicator for NullElkProgressMonitor {
    fn is_canceled(&self) -> bool {
        false
    }
}

impl IElkProgressMonitor for NullElkProgressMonitor {
    fn begin(&mut self, _name: &str, _total_work: f32) -> bool {
        false
    }

    fn worked(&mut self, _work: f32) {}

    fn done(&mut self) {}

    fn is_running(&self) -> bool {
        false
    }

    fn task_name(&self) -> Option<&str> {
        None
    }

    fn sub_task(&mut self, _work: f32) -> Box<dyn IElkProgressMonitor> {
        Box::new(NullElkProgressMonitor)
    }
}

const ROOT_DEBUG_FOLDER_NAME: &str = "logs";
const INFINITE_HIERARCHY_LEVELS: i32 = -1;

pub struct BasicProgressMonitor {
    parent_monitor: Option<*mut BasicProgressMonitor>,
    next_child_index: usize,
    child_index: usize,
    max_levels: i32,
    task_name: Option<String>,
    closed: bool,
    total_work: f32,
    completed_work: f32,
    current_child_work: f32,
    record_logs: bool,
    persist_logs: bool,
    log_messages: Vec<String>,
    log_graphs: Vec<LoggedGraph>,
    debug_folder: Option<PathBuf>,
    log_file: Option<PathBuf>,
    record_execution_time: bool,
    start_time: Option<Instant>,
    total_time: f64,
}

impl Default for BasicProgressMonitor {
    fn default() -> Self {
        BasicProgressMonitor::new()
    }
}

impl BasicProgressMonitor {
    pub fn new() -> Self {
        BasicProgressMonitor {
            parent_monitor: None,
            next_child_index: 0,
            child_index: 0,
            max_levels: INFINITE_HIERARCHY_LEVELS,
            task_name: None,
            closed: false,
            total_work: 0.0,
            completed_work: 0.0,
            current_child_work: -1.0,
            record_logs: false,
            persist_logs: false,
            log_messages: Vec::new(),
            log_graphs: Vec::new(),
            debug_folder: None,
            log_file: None,
            record_execution_time: false,
            start_time: None,
            total_time: 0.0,
        }
    }

    pub fn with_max_hierarchy_levels(&mut self, levels: i32) -> &mut Self {
        if levels < 0 {
            self.max_levels = INFINITE_HIERARCHY_LEVELS;
        } else {
            self.max_levels = levels;
        }
        self
    }

    pub fn with_logging(&mut self, enabled: bool) -> &mut Self {
        self.record_logs = enabled;
        if !enabled {
            self.log_messages.clear();
            self.log_graphs.clear();
        }
        self
    }

    pub fn with_log_persistence(&mut self, enabled: bool) -> &mut Self {
        self.persist_logs = enabled;
        self
    }

    pub fn with_execution_time_measurement(&mut self, enabled: bool) -> &mut Self {
        self.record_execution_time = enabled;
        self
    }

    fn internal_worked(&mut self, work: f32) {
        if self.total_work <= 0.0 || self.completed_work >= self.total_work {
            return;
        }

        self.completed_work += work;
        self.do_worked(self.completed_work, self.total_work);

        if let Some(parent) = self.parent_monitor {
            if self.current_child_work > 0.0 && self.max_levels != 0 {
                let propagated = work / self.total_work * self.current_child_work;
                unsafe {
                    (&mut *parent).internal_worked(propagated);
                }
            }
        }
    }

    fn do_begin(&self, _name: &str, _total_work: f32, _top_instance: bool, _max_levels: i32) {}

    fn do_worked(&self, _completed: f32, _total: f32) {}

    fn do_done(&self, _top_instance: bool, _max_levels: i32) {}

    fn debug_folder_for_root(&mut self) -> Option<PathBuf> {
        let name = self
            .task_name
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or("Unnamed");
        let safe_name = ElkUtil::to_safe_path_name(name);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let mut rng = rand_chars();
        let rand1 = rng.next().unwrap_or('a');
        let rand2 = rng.next().unwrap_or('a');
        let monitor_folder = format!("{timestamp}{rand1}{rand2}-{safe_name}");
        let path = ElkUtil::debug_folder_path(&[ROOT_DEBUG_FOLDER_NAME, &monitor_folder])?;
        Some(PathBuf::from(path))
    }

    fn debug_folder_for_child(&mut self) -> Option<PathBuf> {
        let parent = self.parent_monitor?;
        let parent_ref = unsafe { &mut *parent };
        parent_ref.init_debug_folder(true);
        let index = self.child_index;
        let name = self
            .task_name
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or("Unnamed");
        let safe_name = ElkUtil::to_safe_path_name(name);
        let monitor_folder = format!("{index:02}-{safe_name}");
        let parent_folder = parent_ref.debug_folder();
        parent_folder.map(|path| path.join(monitor_folder))
    }

    fn init_debug_folder(&mut self, ensure_exists: bool) {
        if !self.record_logs || !self.persist_logs || self.debug_folder.is_some() {
            return;
        }

        let folder = if self.parent_monitor.is_some() {
            self.debug_folder_for_child()
        } else {
            self.debug_folder_for_root()
        };
        self.debug_folder = folder;

        if ensure_exists {
            if let Some(path) = self.debug_folder.as_ref() {
                if std::fs::create_dir_all(path).is_err() {
                    self.record_logs = false;
                    self.debug_folder = None;
                }
            }
        }
    }

    fn retrieve_log_file_path(&mut self) -> Option<PathBuf> {
        if self.log_file.is_none() {
            self.log_file = self.retrieve_file_path("log", "txt");
        }
        self.log_file.clone()
    }

    fn retrieve_file_path(&mut self, name: &str, extension: &str) -> Option<PathBuf> {
        self.init_debug_folder(true);
        let folder = self.debug_folder.clone()?;
        let mut path = folder.join(format!("{name}.{extension}"));
        let mut counter = 0usize;
        while path.exists() {
            counter += 1;
            path = folder.join(format!("{name}-{counter}.{extension}"));
        }
        Some(path)
    }
}

impl IElkCancelIndicator for BasicProgressMonitor {
    fn is_canceled(&self) -> bool {
        false
    }
}

impl IElkProgressMonitor for BasicProgressMonitor {
    fn begin(&mut self, name: &str, total_work: f32) -> bool {
        if self.closed {
            panic!("The task is already done.");
        }
        if self.task_name.is_some() {
            return false;
        }
        self.task_name = Some(name.to_string());
        self.total_work = total_work;
        self.do_begin(
            name,
            total_work,
            self.parent_monitor.is_none(),
            self.max_levels,
        );
        if self.record_execution_time {
            self.start_time = Some(Instant::now());
        }
        true
    }

    fn worked(&mut self, work: f32) {
        if work > 0.0 && !self.closed {
            self.internal_worked(work);
        }
    }

    fn done(&mut self) {
        if self.task_name.is_none() {
            panic!("The task has not begun yet.");
        }
        if self.closed {
            return;
        }
        if self.record_execution_time {
            if let Some(start) = self.start_time {
                self.total_time = start.elapsed().as_secs_f64();
            }
        }
        if self.completed_work < self.total_work {
            let remaining = self.total_work - self.completed_work;
            self.internal_worked(remaining);
        }
        self.do_done(self.parent_monitor.is_none(), self.max_levels);
        self.closed = true;
    }

    fn is_running(&self) -> bool {
        self.task_name.is_some() && !self.closed
    }

    fn task_name(&self) -> Option<&str> {
        self.task_name.as_deref()
    }

    fn sub_task(&mut self, work: f32) -> Box<dyn IElkProgressMonitor> {
        if self.closed {
            return Box::new(NullElkProgressMonitor);
        }
        let mut sub = BasicProgressMonitor::new();
        let new_levels = if self.max_levels > 0 {
            self.max_levels - 1
        } else {
            self.max_levels
        };
        sub.with_max_hierarchy_levels(new_levels)
            .with_logging(self.record_logs)
            .with_log_persistence(self.persist_logs)
            .with_execution_time_measurement(self.record_execution_time);
        sub.parent_monitor = Some(self as *mut _);
        sub.child_index = self.next_child_index;
        self.next_child_index = self.next_child_index.saturating_add(1);
        self.current_child_work = work;
        Box::new(sub)
    }

    fn is_logging_enabled(&self) -> bool {
        self.record_logs
    }

    fn is_log_persistence_enabled(&self) -> bool {
        self.persist_logs
    }

    fn log(&mut self, message: &str) {
        if !self.record_logs {
            return;
        }
        self.log_messages.push(message.to_string());
        if self.persist_logs {
            if let Some(path) = self.retrieve_log_file_path() {
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .and_then(|mut file| {
                        use std::io::Write;
                        writeln!(file, "{}", message)
                    });
            }
        }
    }

    fn logs(&self) -> Vec<String> {
        self.log_messages.clone()
    }

    fn log_graph(&mut self, _graph: &ElkNodeRef, tag: &str) {
        self.log_graph_typed(_graph, tag, LoggedGraphType::Elk);
    }

    fn log_graph_typed(
        &mut self,
        _graph: &dyn std::any::Any,
        tag: &str,
        graph_type: LoggedGraphType,
    ) {
        if !self.record_logs {
            return;
        }
        let actual_tag = if tag.is_empty() { "Unnamed" } else { tag };
        self.log_graphs
            .push(LoggedGraph::new(actual_tag, graph_type));

        if self.persist_logs {
            let extension = match graph_type {
                LoggedGraphType::Elk => "elkg",
                LoggedGraphType::Json => "json",
                LoggedGraphType::Dot => "dot",
                LoggedGraphType::Svg => "svg",
            };
            if let Some(path) = self.retrieve_file_path(actual_tag, extension) {
                let _ = std::fs::write(path, format!("Logged graph: {actual_tag}"));
            }
        }
    }

    fn logged_graphs(&self) -> Vec<LoggedGraph> {
        self.log_graphs.clone()
    }

    fn debug_folder(&self) -> Option<PathBuf> {
        self.debug_folder.clone()
    }

    fn is_execution_time_measured(&self) -> bool {
        self.record_execution_time
    }

    fn execution_time(&self) -> f64 {
        self.total_time
    }
}

fn rand_chars() -> impl Iterator<Item = char> {
    let mut rng = rand_seed();
    std::iter::from_fn(move || {
        let value = (rng % 26) as u8;
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        Some((b'a' + value) as char)
    })
}

fn rand_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    nanos as u64
}
