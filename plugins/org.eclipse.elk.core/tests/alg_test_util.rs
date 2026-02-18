#![allow(dead_code)]

use std::any::Any;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use org_eclipse_elk_core::org::eclipse::elk::core::alg::{
    ILayoutPhase, ILayoutPhaseFactory, ILayoutProcessor, ILayoutProcessorFactory,
    LayoutProcessorConfiguration,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSetType, IElkProgressMonitor};

pub type TestGraph = String;
pub type PhaseFactory = Arc<dyn ILayoutPhaseFactory<TestPhases, TestGraph>>;
pub type ProcessorFactory = Arc<dyn ILayoutProcessorFactory<TestGraph>>;

static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

static PHASE_CREATE_COUNTS: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
static PROCESSOR_CREATE_COUNTS: [AtomicUsize; 3] = [
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
];

static PHASE_FACTORIES: LazyLock<[PhaseFactory; 2]> = LazyLock::new(|| {
    [
        Arc::new(TestPhases::Phase1) as PhaseFactory,
        Arc::new(TestPhases::Phase2) as PhaseFactory,
    ]
});

static PROCESSOR_FACTORIES: LazyLock<[ProcessorFactory; 3]> = LazyLock::new(|| {
    [
        Arc::new(TestProcessors::Processor1) as ProcessorFactory,
        Arc::new(TestProcessors::Processor2) as ProcessorFactory,
        Arc::new(TestProcessors::Processor3) as ProcessorFactory,
    ]
});

pub fn lock_tests() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap()
}

pub fn reset_create_counts() {
    for counter in PHASE_CREATE_COUNTS.iter() {
        counter.store(0, Ordering::SeqCst);
    }
    for counter in PROCESSOR_CREATE_COUNTS.iter() {
        counter.store(0, Ordering::SeqCst);
    }
}

pub fn phase_create_counts() -> [usize; 2] {
    [
        PHASE_CREATE_COUNTS[0].load(Ordering::SeqCst),
        PHASE_CREATE_COUNTS[1].load(Ordering::SeqCst),
    ]
}

pub fn processor_create_counts() -> [usize; 3] {
    [
        PROCESSOR_CREATE_COUNTS[0].load(Ordering::SeqCst),
        PROCESSOR_CREATE_COUNTS[1].load(Ordering::SeqCst),
        PROCESSOR_CREATE_COUNTS[2].load(Ordering::SeqCst),
    ]
}

pub fn phase_factory(phase: TestPhases) -> PhaseFactory {
    PHASE_FACTORIES[phase.ordinal()].clone()
}

pub fn processor_factory(processor: TestProcessors) -> ProcessorFactory {
    PROCESSOR_FACTORIES[processor.ordinal()].clone()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TestPhases {
    Phase1,
    Phase2,
}

impl TestPhases {
    pub fn ordinal(self) -> usize {
        match self {
            TestPhases::Phase1 => 0,
            TestPhases::Phase2 => 1,
        }
    }
}

impl EnumSetType for TestPhases {
    fn variants() -> &'static [Self] {
        static VARIANTS: [TestPhases; 2] = [TestPhases::Phase1, TestPhases::Phase2];
        &VARIANTS
    }
}

impl fmt::Display for TestPhases {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestPhases::Phase1 => write!(f, "PHASE_1"),
            TestPhases::Phase2 => write!(f, "PHASE_2"),
        }
    }
}

impl ILayoutPhaseFactory<TestPhases, TestGraph> for TestPhases {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<TestPhases, TestGraph>> {
        PHASE_CREATE_COUNTS[self.ordinal()].fetch_add(1, Ordering::SeqCst);
        Box::new(TestPhaseImpl { phase: *self })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}

struct TestPhaseImpl {
    phase: TestPhases,
}

impl ILayoutPhase<TestPhases, TestGraph> for TestPhaseImpl {
    fn process(&mut self, graph: &mut TestGraph, _progress_monitor: &mut dyn IElkProgressMonitor) {
        graph.push_str(&self.phase.to_string());
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &TestGraph,
    ) -> Option<LayoutProcessorConfiguration<TestPhases, TestGraph>> {
        let mut config = LayoutProcessorConfiguration::create();
        config.add_before(
            self.phase,
            processor_factory(TestProcessors::from_phase(self.phase)),
        );
        Some(config)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TestProcessors {
    Processor1,
    Processor2,
    Processor3,
}

impl TestProcessors {
    pub fn ordinal(self) -> usize {
        match self {
            TestProcessors::Processor1 => 0,
            TestProcessors::Processor2 => 1,
            TestProcessors::Processor3 => 2,
        }
    }

    pub fn from_phase(phase: TestPhases) -> Self {
        match phase {
            TestPhases::Phase1 => TestProcessors::Processor1,
            TestPhases::Phase2 => TestProcessors::Processor2,
        }
    }
}

impl fmt::Display for TestProcessors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestProcessors::Processor1 => write!(f, "PROCESSOR_1"),
            TestProcessors::Processor2 => write!(f, "PROCESSOR_2"),
            TestProcessors::Processor3 => write!(f, "PROCESSOR_3"),
        }
    }
}

impl ILayoutProcessorFactory<TestGraph> for TestProcessors {
    fn create(&self) -> Box<dyn ILayoutProcessor<TestGraph>> {
        PROCESSOR_CREATE_COUNTS[self.ordinal()].fetch_add(1, Ordering::SeqCst);
        Box::new(TestProcessorImpl { processor: *self })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}

struct TestProcessorImpl {
    processor: TestProcessors,
}

impl ILayoutProcessor<TestGraph> for TestProcessorImpl {
    fn process(&mut self, graph: &mut TestGraph, _progress_monitor: &mut dyn IElkProgressMonitor) {
        graph.push_str(&self.processor.to_string());
    }
}
