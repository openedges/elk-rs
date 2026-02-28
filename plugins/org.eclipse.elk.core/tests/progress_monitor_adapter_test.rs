use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::service::util::{
    IProgressMonitor, ProgressMonitorAdapter,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

#[derive(Default)]
struct TestMonitorState {
    begin_calls: i32,
    worked: i32,
    done_calls: i32,
    sub_tasks: i32,
    canceled: bool,
}

#[derive(Clone)]
struct TestMonitor {
    state: Arc<Mutex<TestMonitorState>>,
}

impl IProgressMonitor for TestMonitor {
    fn begin_task(&self, _name: &str, _total_work: i32) {
        self.state.lock().unwrap().begin_calls += 1;
    }

    fn sub_task(&self, _name: &str) {
        self.state.lock().unwrap().sub_tasks += 1;
    }

    fn worked(&self, work: i32) {
        self.state.lock().unwrap().worked += work;
    }

    fn done(&self) {
        self.state.lock().unwrap().done_calls += 1;
    }

    fn is_canceled(&self) -> bool {
        self.state.lock().unwrap().canceled
    }
}

#[test]
fn progress_monitor_adapter_reports_work() {
    let state = Arc::new(Mutex::new(TestMonitorState::default()));
    let monitor = TestMonitor {
        state: state.clone(),
    };
    let mut adapter = ProgressMonitorAdapter::new(Arc::new(monitor));

    assert!(adapter.begin("task", 5.0));
    adapter.worked(2.0);
    adapter.worked(3.0);
    adapter.done();

    let snapshot = state.lock().unwrap();
    assert_eq!(snapshot.begin_calls, 1);
    assert_eq!(snapshot.worked, 5);
    assert_eq!(snapshot.done_calls, 1);
}

#[test]
fn progress_monitor_adapter_subtask_reports_sub_task() {
    let state = Arc::new(Mutex::new(TestMonitorState::default()));
    let monitor = TestMonitor {
        state: state.clone(),
    };
    let mut adapter = ProgressMonitorAdapter::new(Arc::new(monitor));

    adapter.begin("parent", 1.0);
    let mut child = adapter.sub_task(1.0);
    child.begin("child", 1.0);

    let snapshot = state.lock().unwrap();
    assert_eq!(snapshot.sub_tasks, 1);
}
