use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    BasicProgressMonitor, IElkProgressMonitor,
};

#[test]
fn basic_progress_monitor_tracks_logs_and_time() {
    let mut monitor = BasicProgressMonitor::new();
    monitor
        .with_logging(true)
        .with_execution_time_measurement(true);

    assert!(monitor.begin("Task", 10.0));
    monitor.worked(5.0);
    monitor.log("hello");
    monitor.done();

    assert!(!monitor.is_running());
    assert_eq!(Some("Task"), monitor.task_name());
    let logs = monitor.logs();
    assert_eq!(1, logs.len());
    assert_eq!("hello", logs[0]);
    assert!(monitor.execution_time() >= 0.0);
}
