use crate::common::alg_test_util::{processor_factory, TestPhases, TestProcessors};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::LayoutProcessorConfiguration;

#[test]
fn test_before() {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .before(TestPhases::Phase1)
        .add(processor_factory(TestProcessors::Processor1))
        .before(TestPhases::Phase2)
        .add(processor_factory(TestProcessors::Processor2))
        .add(processor_factory(TestProcessors::Processor3));

    assert_eq!(1, config.processors_before(TestPhases::Phase1).len());
    assert_eq!(2, config.processors_after(TestPhases::Phase1).len());
    assert_eq!(2, config.processors_before(TestPhases::Phase2).len());
    assert_eq!(0, config.processors_after(TestPhases::Phase2).len());
}

#[test]
fn test_after() {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .after(TestPhases::Phase1)
        .add(processor_factory(TestProcessors::Processor1))
        .after(TestPhases::Phase2)
        .add(processor_factory(TestProcessors::Processor2))
        .add(processor_factory(TestProcessors::Processor3));

    assert_eq!(0, config.processors_before(TestPhases::Phase1).len());
    assert_eq!(1, config.processors_after(TestPhases::Phase1).len());
    assert_eq!(1, config.processors_before(TestPhases::Phase2).len());
    assert_eq!(2, config.processors_after(TestPhases::Phase2).len());
}

#[test]
fn test_add_before() {
    let mut config = LayoutProcessorConfiguration::create();
    config.before(TestPhases::Phase1);
    config.add_before(
        TestPhases::Phase2,
        processor_factory(TestProcessors::Processor1),
    );

    assert_eq!(0, config.processors_before(TestPhases::Phase1).len());
    assert_eq!(1, config.processors_after(TestPhases::Phase1).len());
    assert_eq!(1, config.processors_before(TestPhases::Phase2).len());
    assert_eq!(0, config.processors_after(TestPhases::Phase2).len());
}

#[test]
#[should_panic]
fn test_add_before_panics() {
    let mut config = LayoutProcessorConfiguration::create();
    config.before(TestPhases::Phase1);
    config.add_before(
        TestPhases::Phase2,
        processor_factory(TestProcessors::Processor1),
    );
    config.add(processor_factory(TestProcessors::Processor2));
}

#[test]
fn test_add_after() {
    let mut config = LayoutProcessorConfiguration::create();
    config.before(TestPhases::Phase1);
    config.add_after(
        TestPhases::Phase1,
        processor_factory(TestProcessors::Processor1),
    );

    assert_eq!(0, config.processors_before(TestPhases::Phase1).len());
    assert_eq!(1, config.processors_after(TestPhases::Phase1).len());
    assert_eq!(1, config.processors_before(TestPhases::Phase2).len());
    assert_eq!(0, config.processors_after(TestPhases::Phase2).len());
}

#[test]
#[should_panic]
fn test_add_after_panics() {
    let mut config = LayoutProcessorConfiguration::create();
    config.before(TestPhases::Phase1);
    config.add_after(
        TestPhases::Phase1,
        processor_factory(TestProcessors::Processor1),
    );
    config.add(processor_factory(TestProcessors::Processor2));
}

#[test]
fn test_add_all() {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .after(TestPhases::Phase1)
        .add(processor_factory(TestProcessors::Processor1))
        .after(TestPhases::Phase2)
        .add(processor_factory(TestProcessors::Processor2))
        .add(processor_factory(TestProcessors::Processor3));

    let mut config2 = LayoutProcessorConfiguration::create();
    config2.add_all(&config);

    assert_eq!(0, config2.processors_before(TestPhases::Phase1).len());
    assert_eq!(1, config2.processors_after(TestPhases::Phase1).len());
    assert_eq!(1, config2.processors_before(TestPhases::Phase2).len());
    assert_eq!(2, config2.processors_after(TestPhases::Phase2).len());
}
