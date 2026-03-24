use crate::common::alg_test_util::{
    lock_tests, phase_create_counts, phase_factory, processor_create_counts, processor_factory,
    reset_create_counts, TestGraph, TestPhases, TestProcessors,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::{
    AlgorithmAssembler, LayoutProcessorConfiguration,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

#[test]
fn test_enable_caching() {
    let _guard = lock_tests();
    reset_create_counts();

    let mut assembler: AlgorithmAssembler<TestPhases, TestGraph> = AlgorithmAssembler::create();
    assembler.set_phase(TestPhases::Phase1, phase_factory(TestPhases::Phase1));
    assembler.set_phase(TestPhases::Phase2, phase_factory(TestPhases::Phase2));

    let graph = String::new();
    let algorithm = assembler.build(&graph);
    assert_eq!(4, algorithm.len());

    let phase_counts = phase_create_counts();
    let processor_counts = processor_create_counts();

    let algorithm2 = assembler.build(&graph);
    assert_eq!(4, algorithm2.len());

    assert_eq!(phase_counts, phase_create_counts());
    assert_eq!(processor_counts, processor_create_counts());
}

#[test]
fn test_disable_caching() {
    let _guard = lock_tests();
    reset_create_counts();

    let mut assembler: AlgorithmAssembler<TestPhases, TestGraph> = AlgorithmAssembler::create();
    assembler.with_caching(false);
    assembler.set_phase(TestPhases::Phase1, phase_factory(TestPhases::Phase1));
    assembler.set_phase(TestPhases::Phase2, phase_factory(TestPhases::Phase2));

    let graph = String::new();
    let _ = assembler.build(&graph);

    let phase_counts = phase_create_counts();
    let processor_counts = processor_create_counts();

    let _ = assembler.build(&graph);

    let phase_counts_after = phase_create_counts();
    let processor_counts_after = processor_create_counts();

    assert!(phase_counts_after[0] > phase_counts[0]);
    assert!(phase_counts_after[1] > phase_counts[1]);
    assert!(processor_counts_after[0] > processor_counts[0]);
    assert!(processor_counts_after[1] > processor_counts[1]);
}

#[test]
#[should_panic]
fn test_fail_on_missing_phase() {
    let _guard = lock_tests();
    let mut assembler: AlgorithmAssembler<TestPhases, TestGraph> = AlgorithmAssembler::create();
    assembler.set_phase(TestPhases::Phase1, phase_factory(TestPhases::Phase1));

    let graph = String::new();
    assembler.build(&graph);
}

#[test]
fn test_dont_fail_on_missing_phase() {
    let _guard = lock_tests();
    let mut assembler: AlgorithmAssembler<TestPhases, TestGraph> = AlgorithmAssembler::create();
    assembler.with_fail_on_missing_phase(false);
    assembler.set_phase(TestPhases::Phase1, phase_factory(TestPhases::Phase1));

    let graph = String::new();
    let algorithm = assembler.build(&graph);
    assert!(!algorithm.is_empty());
}

#[test]
fn test_clear_cache() {
    let _guard = lock_tests();
    reset_create_counts();

    let mut assembler: AlgorithmAssembler<TestPhases, TestGraph> = AlgorithmAssembler::create();
    assembler.set_phase(TestPhases::Phase1, phase_factory(TestPhases::Phase1));
    assembler.set_phase(TestPhases::Phase2, phase_factory(TestPhases::Phase2));

    let graph = String::new();
    let _ = assembler.build(&graph);

    let phase_counts = phase_create_counts();
    let processor_counts = processor_create_counts();

    assembler.clear_cache();

    let _ = assembler.build(&graph);

    let phase_counts_after = phase_create_counts();
    let processor_counts_after = processor_create_counts();

    assert!(phase_counts_after[0] > phase_counts[0]);
    assert!(phase_counts_after[1] > phase_counts[1]);
    assert!(processor_counts_after[0] > processor_counts[0]);
    assert!(processor_counts_after[1] > processor_counts[1]);
}

#[test]
#[should_panic]
fn test_reset_with_fail_on_missing_phase() {
    let _guard = lock_tests();
    let mut assembler: AlgorithmAssembler<TestPhases, TestGraph> = AlgorithmAssembler::create();
    assembler.with_fail_on_missing_phase(true);
    assembler.set_phase(TestPhases::Phase1, phase_factory(TestPhases::Phase1));
    assembler.set_phase(TestPhases::Phase2, phase_factory(TestPhases::Phase2));

    assembler.reset();

    let graph = String::new();
    assembler.build(&graph);
}

#[test]
fn test_reset_without_fail_on_missing_phase() {
    let _guard = lock_tests();
    let mut assembler: AlgorithmAssembler<TestPhases, TestGraph> = AlgorithmAssembler::create();
    assembler.with_fail_on_missing_phase(false);
    assembler.set_phase(TestPhases::Phase1, phase_factory(TestPhases::Phase1));
    assembler.set_phase(TestPhases::Phase2, phase_factory(TestPhases::Phase2));

    assembler.reset();

    let graph = String::new();
    let algorithm = assembler.build(&graph);
    assert_eq!(0, algorithm.len());
}

#[test]
fn test_add_processor_configuration() {
    let _guard = lock_tests();
    let mut assembler: AlgorithmAssembler<TestPhases, TestGraph> = AlgorithmAssembler::create();
    let mut config = LayoutProcessorConfiguration::create();
    config.add_before(
        TestPhases::Phase1,
        processor_factory(TestProcessors::Processor3),
    );
    assembler.add_processor_configuration(&config);

    assembler.set_phase(TestPhases::Phase1, phase_factory(TestPhases::Phase1));
    assembler.set_phase(TestPhases::Phase2, phase_factory(TestPhases::Phase2));

    let graph = String::new();
    let algorithm = assembler.build(&graph);

    let expected = [
        "PROCESSOR_1",
        "PROCESSOR_3",
        "PHASE_1",
        "PROCESSOR_2",
        "PHASE_2",
    ];

    for (processor, expected) in algorithm.iter().zip(expected.iter()) {
        let mut buffer = String::new();
        let mut monitor = NullElkProgressMonitor;
        processor
            .lock()
            
            .process(&mut buffer, &mut monitor);
        assert_eq!(*expected, buffer);
    }
}
