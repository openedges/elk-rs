use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use crate::org::eclipse::elk::core::util::{EnumSetType, IElkProgressMonitor};

use super::enum_based_factory_comparator::EnumBasedFactoryComparator;
use super::i_layout_phase::ILayoutPhase;
use super::i_layout_phase_factory::ILayoutPhaseFactory;
use super::i_layout_processor::ILayoutProcessor;
use super::layout_processor_configuration::{LayoutProcessorConfiguration, ProcessorFactory};

type PhaseFactory<P, G> = Arc<dyn ILayoutPhaseFactory<P, G>>;
type PhaseHandle<P, G> = Rc<RefCell<Box<dyn ILayoutPhase<P, G>>>>;
type ProcessorHandle<G> = Rc<RefCell<Box<dyn ILayoutProcessor<G>>>>;

type ProcessorComparator<G> =
    Arc<dyn Fn(&ProcessorFactory<G>, &ProcessorFactory<G>) -> Ordering + Send + Sync>;

pub struct AlgorithmAssembler<P, G>
where
    P: EnumSetType,
{
    enable_caching: bool,
    fail_on_missing_phase: bool,
    processor_comparator: ProcessorComparator<G>,
    phases: Vec<Option<PhaseFactory<P, G>>>,
    configured_phases: BTreeSet<P>,
    additional_processors: LayoutProcessorConfiguration<P, G>,
    phase_cache: HashMap<usize, PhaseHandle<P, G>>,
    processor_cache: HashMap<usize, ProcessorHandle<G>>,
    phase_variants: Vec<P>,
    _phantom: PhantomData<G>,
}

impl<P, G> AlgorithmAssembler<P, G>
where
    P: EnumSetType,
    G: 'static,
{
    pub fn create() -> Self {
        let variants = P::variants();
        if variants.is_empty() {
            panic!("There must be at least one phase in the phase enumeration.");
        }

        AlgorithmAssembler {
            enable_caching: true,
            fail_on_missing_phase: true,
            processor_comparator: {
                let comparator = EnumBasedFactoryComparator;
                Arc::new(move |a: &ProcessorFactory<G>, b: &ProcessorFactory<G>| {
                    comparator.compare(a, b)
                })
            },
            phases: vec![None; variants.len()],
            configured_phases: BTreeSet::new(),
            additional_processors: LayoutProcessorConfiguration::create(),
            phase_cache: HashMap::new(),
            processor_cache: HashMap::new(),
            phase_variants: variants.to_vec(),
            _phantom: PhantomData,
        }
    }

    pub fn with_caching(&mut self, enabled: bool) -> &mut Self {
        self.enable_caching = enabled;
        self
    }

    pub fn with_fail_on_missing_phase(&mut self, fail: bool) -> &mut Self {
        self.fail_on_missing_phase = fail;
        self
    }

    pub fn with_processor_comparator<F>(&mut self, comparator: F) -> &mut Self
    where
        F: Fn(&ProcessorFactory<G>, &ProcessorFactory<G>) -> Ordering + Send + Sync + 'static,
    {
        self.processor_comparator = Arc::new(comparator);
        self
    }

    pub fn clear_cache(&mut self) -> &mut Self {
        self.phase_cache.clear();
        self.processor_cache.clear();
        self
    }

    pub fn reset(&mut self) -> &mut Self {
        self.phases.fill(None);
        self.configured_phases.clear();
        self.additional_processors.clear();
        self
    }

    pub fn set_phase(&mut self, phase: P, phase_factory: PhaseFactory<P, G>) -> &mut Self {
        let index = self.phase_index(phase);
        self.phases[index] = Some(phase_factory);
        self.configured_phases.insert(phase);
        self
    }

    pub fn add_processor_configuration(
        &mut self,
        config: &LayoutProcessorConfiguration<P, G>,
    ) -> &mut Self {
        self.additional_processors.add_all(config);
        self
    }

    pub fn build(&mut self, graph: &G) -> Vec<Box<dyn ILayoutProcessor<G>>> {
        if self.fail_on_missing_phase && self.configured_phases.len() < self.phase_variants.len()
        {
            panic!(
                "Expected {} phases to be configured; only found {}",
                self.phase_variants.len(),
                self.configured_phases.len()
            );
        }

        let variants = self.phase_variants.clone();
        let mut phases: Vec<Option<PhaseHandle<P, G>>> =
            Vec::with_capacity(variants.len());
        for phase in &variants {
            let index = self.phase_index(*phase);
            let factory = self.phases[index].clone();
            if let Some(factory) = factory {
                phases.push(Some(self.retrieve_phase(&factory)));
            } else {
                phases.push(None);
            }
        }

        let mut processor_configuration = LayoutProcessorConfiguration::create();
        for phase in phases.iter().flatten() {
            if let Some(config) = phase.borrow().get_layout_processor_configuration(graph) {
                processor_configuration.add_all(&config);
            }
        }
        processor_configuration.add_all(&self.additional_processors);

        let mut algorithm: Vec<Box<dyn ILayoutProcessor<G>>> = Vec::new();

        let variants = self.phase_variants.clone();
        for phase in &variants {
            let processors = processor_configuration.processors_before(*phase);
            algorithm.extend(self.retrieve_processors(processors));

            let phase_index = self.phase_index(*phase);
            if let Some(phase) = &phases[phase_index] {
                algorithm.push(Box::new(PhaseProcessorAdapter {
                    phase: phase.clone(),
                }));
            }
        }

        if let Some(last_phase) = variants.last().copied() {
            let processors = processor_configuration.processors_after(last_phase);
            algorithm.extend(self.retrieve_processors(processors));
        }

        algorithm
    }

    fn retrieve_processors(
        &mut self,
        factories: Vec<ProcessorFactory<G>>,
    ) -> Vec<Box<dyn ILayoutProcessor<G>>> {
        let mut sorted = factories;
        let comparator = self.processor_comparator.clone();
        sorted.sort_by(|a, b| (comparator)(a, b));

        let mut processors: Vec<Box<dyn ILayoutProcessor<G>>> = Vec::with_capacity(sorted.len());
        for factory in sorted {
            let processor = self.retrieve_processor(&factory);
            processors.push(Box::new(SharedProcessorAdapter { processor }));
        }
        processors
    }

    fn retrieve_phase(
        &mut self,
        factory: &PhaseFactory<P, G>,
    ) -> PhaseHandle<P, G> {
        if !self.enable_caching {
            return Rc::new(RefCell::new(factory.create_phase()));
        }

        let key = phase_factory_key(factory);
        if let Some(existing) = self.phase_cache.get(&key) {
            return existing.clone();
        }

        let phase = Rc::new(RefCell::new(factory.create_phase()));
        self.phase_cache.insert(key, phase.clone());
        phase
    }

    fn retrieve_processor(
        &mut self,
        factory: &ProcessorFactory<G>,
    ) -> ProcessorHandle<G> {
        if !self.enable_caching {
            return Rc::new(RefCell::new(factory.create()));
        }

        let key = processor_factory_key(factory);
        if let Some(existing) = self.processor_cache.get(&key) {
            return existing.clone();
        }

        let processor = Rc::new(RefCell::new(factory.create()));
        self.processor_cache.insert(key, processor.clone());
        processor
    }

    fn phase_index(&self, phase: P) -> usize {
        self.phase_variants
            .iter()
            .position(|candidate| *candidate == phase)
            .unwrap_or_else(|| panic!("Phase not found in variants."))
    }
}

struct SharedProcessorAdapter<G> {
    processor: ProcessorHandle<G>,
}

impl<G> ILayoutProcessor<G> for SharedProcessorAdapter<G> {
    fn process(&mut self, graph: &mut G, progress_monitor: &mut dyn IElkProgressMonitor) {
        self.processor.borrow_mut().process(graph, progress_monitor);
    }
}

struct PhaseProcessorAdapter<P, G> {
    phase: PhaseHandle<P, G>,
}

impl<P, G> ILayoutProcessor<G> for PhaseProcessorAdapter<P, G>
where
    P: EnumSetType,
{
    fn process(&mut self, graph: &mut G, progress_monitor: &mut dyn IElkProgressMonitor) {
        self.phase.borrow_mut().process(graph, progress_monitor);
    }
}

fn processor_factory_key<G>(factory: &ProcessorFactory<G>) -> usize {
    Arc::as_ptr(factory) as *const () as usize
}

fn phase_factory_key<P, G>(factory: &PhaseFactory<P, G>) -> usize {
    Arc::as_ptr(factory) as *const () as usize
}
