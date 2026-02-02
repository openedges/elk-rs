use std::marker::PhantomData;
use std::sync::Arc;

use crate::org::eclipse::elk::core::util::AbstractRandomListAccessor;
use crate::org::eclipse::elk::core::util::EnumSetType;

use super::i_layout_processor_factory::ILayoutProcessorFactory;

pub type ProcessorFactory<G> = Arc<dyn ILayoutProcessorFactory<G>>;

pub struct LayoutProcessorConfiguration<P, G>
where
    P: EnumSetType,
{
    list: Vec<Vec<ProcessorFactory<G>>>,
    current_index: Option<usize>,
    _phantom: PhantomData<P>,
}

impl<P, G> LayoutProcessorConfiguration<P, G>
where
    P: EnumSetType,
{
    pub fn create() -> Self {
        LayoutProcessorConfiguration {
            list: Vec::new(),
            current_index: None,
            _phantom: PhantomData,
        }
    }

    pub fn create_from(source: &LayoutProcessorConfiguration<P, G>) -> Self {
        LayoutProcessorConfiguration {
            list: source.list.clone(),
            current_index: source.current_index,
            _phantom: PhantomData,
        }
    }

    pub fn clear(&mut self) -> &mut Self {
        self.clear_list();
        self.current_index = None;
        self
    }

    pub fn before(&mut self, phase: P) -> &mut Self {
        self.current_index = Some(phase_index(phase));
        self
    }

    pub fn after(&mut self, phase: P) -> &mut Self {
        self.current_index = Some(phase_index(phase) + 1);
        self
    }

    pub fn add(&mut self, processor: ProcessorFactory<G>) -> &mut Self {
        let Some(index) = self.current_index else {
            panic!("Did not call before(...) or after(...) before calling add(...).");
        };
        self.do_add(index, processor);
        self
    }

    pub fn add_before(&mut self, phase: P, processor: ProcessorFactory<G>) -> &mut Self {
        self.current_index = None;
        let index = phase_index(phase);
        self.do_add(index, processor);
        self
    }

    pub fn add_after(&mut self, phase: P, processor: ProcessorFactory<G>) -> &mut Self {
        self.current_index = None;
        let index = phase_index(phase) + 1;
        self.do_add(index, processor);
        self
    }

    pub fn add_all(&mut self, configuration: &LayoutProcessorConfiguration<P, G>) -> &mut Self {
        for index in 0..configuration.get_list_size() {
            let slot = configuration.list.get(index).cloned().unwrap_or_default();
            for processor in slot {
                self.do_add(index, processor);
            }
        }
        self
    }

    pub fn processors_before(&mut self, phase: P) -> Vec<ProcessorFactory<G>> {
        self.processors(phase_index(phase))
    }

    pub fn processors_after(&mut self, phase: P) -> Vec<ProcessorFactory<G>> {
        self.processors(phase_index(phase) + 1)
    }

    fn processors(&mut self, index: usize) -> Vec<ProcessorFactory<G>> {
        self.get_list_item(index).clone()
    }

    fn do_add(&mut self, index: usize, processor: ProcessorFactory<G>) {
        let slot = self.get_list_item(index);
        if slot.iter().any(|item| Arc::ptr_eq(item, &processor)) {
            return;
        }
        slot.push(processor);
    }
}

impl<P, G> Default for LayoutProcessorConfiguration<P, G>
where
    P: EnumSetType,
{
    fn default() -> Self {
        Self::create()
    }
}

impl<P, G> AbstractRandomListAccessor<Vec<ProcessorFactory<G>>> for LayoutProcessorConfiguration<P, G>
where
    P: EnumSetType,
{
    fn list(&self) -> &Vec<Vec<ProcessorFactory<G>>> {
        &self.list
    }

    fn list_mut(&mut self) -> &mut Vec<Vec<ProcessorFactory<G>>> {
        &mut self.list
    }

    fn provide_default(&self) -> Vec<ProcessorFactory<G>> {
        Vec::new()
    }
}

fn phase_index<P: EnumSetType>(phase: P) -> usize {
    P::variants()
        .iter()
        .position(|candidate| *candidate == phase)
        .unwrap_or_else(|| panic!("Phase not found in variants."))
}
