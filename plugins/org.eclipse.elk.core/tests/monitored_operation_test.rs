use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use org_eclipse_elk_core::org::eclipse::elk::core::service::util::{
    IMonitoredOperation, MonitoredOperation, OperationStatus,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

struct DummyOperation {
    called: Arc<AtomicBool>,
    expect_cancel: bool,
}

impl IMonitoredOperation for DummyOperation {
    fn execute(&mut self, monitor: &mut dyn IElkProgressMonitor) -> OperationStatus {
        self.called.store(true, Ordering::SeqCst);
        if self.expect_cancel && monitor.is_canceled() {
            return OperationStatus::Canceled;
        }
        OperationStatus::Ok
    }
}

struct AlwaysCancel;

impl org_eclipse_elk_core::org::eclipse::elk::core::util::IElkCancelIndicator for AlwaysCancel {
    fn is_canceled(&self) -> bool {
        true
    }
}

#[test]
fn monitored_operation_runs() {
    let called = Arc::new(AtomicBool::new(false));
    let mut operation = DummyOperation {
        called: called.clone(),
        expect_cancel: false,
    };
    let mut runner = MonitoredOperation::new(None);
    let status = runner.run_monitored(&mut operation);
    assert_eq!(status, OperationStatus::Ok);
    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn monitored_operation_respects_cancel_indicator() {
    let called = Arc::new(AtomicBool::new(false));
    let mut operation = DummyOperation {
        called: called.clone(),
        expect_cancel: true,
    };
    let indicator = Arc::new(AlwaysCancel);
    let mut runner = MonitoredOperation::new(Some(indicator));
    let status = runner.run_monitored(&mut operation);
    assert_eq!(status, OperationStatus::Canceled);
    assert!(!called.load(Ordering::SeqCst));
}
