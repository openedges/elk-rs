use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use crate::org::eclipse::elk::core::data::LayoutAlgorithmData;


pub trait ILayoutExecutionListener {
    fn layout_processor_ready(&self, processor: &dyn Any, graph: &dyn Any, is_root: bool);
    fn layout_processor_finished(&self, processor: &dyn Any, graph: &dyn Any, is_root: bool);
}

pub struct TestController {
    layout_algorithm_id: String,
    installed: Cell<bool>,
    listeners: RefCell<Vec<Rc<dyn ILayoutExecutionListener>>>,
}

impl TestController {
    pub fn new(layout_algorithm_id: impl Into<String>) -> Box<Self> {
        Box::new(TestController {
            layout_algorithm_id: layout_algorithm_id.into(),
            installed: Cell::new(false),
            listeners: RefCell::new(Vec::new()),
        })
    }

    pub fn get_target_algorithm_id(&self) -> &str {
        &self.layout_algorithm_id
    }

    pub fn targets(&self, algorithm_data: &LayoutAlgorithmData) -> bool {
        algorithm_data.id() == self.layout_algorithm_id
    }

    pub fn install(
        &mut self,
        layout_provider: &mut dyn AbstractLayoutProvider,
    ) -> Result<(), String> {
        if self.installed.get() {
            return Err("Test controller may be installed on only one layout provider at a time".to_string());
        }

        let Some(testable) = layout_provider.as_white_box_testable() else {
            return Err(
                "Test controllers can only be installed on white-box testable layout algorithms".to_string(),
            );
        };

        let controller_ptr: *mut TestController = self;
        testable.set_test_controller(Some(controller_ptr));
        self.installed.set(true);
        Ok(())
    }

    pub fn uninstall_from(&mut self, layout_provider: &mut dyn AbstractLayoutProvider) {
        if let Some(testable) = layout_provider.as_white_box_testable() {
            testable.set_test_controller(None);
        }
        self.installed.set(false);
    }

    pub fn notify_processor_ready(
        &self,
        graph: &dyn Any,
        processor: &dyn Any,
        is_root: bool,
    ) {
        for listener in self.listeners.borrow().iter() {
            listener.layout_processor_ready(processor, graph, is_root);
        }
    }

    pub fn notify_processor_finished(
        &self,
        graph: &dyn Any,
        processor: &dyn Any,
        is_root: bool,
    ) {
        for listener in self.listeners.borrow().iter() {
            listener.layout_processor_finished(processor, graph, is_root);
        }
    }

    pub fn add_layout_execution_listener(&self, listener: Rc<dyn ILayoutExecutionListener>) {
        if self
            .listeners
            .borrow()
            .iter()
            .any(|existing| Rc::ptr_eq(existing, &listener))
        {
            return;
        }
        self.listeners.borrow_mut().push(listener);
    }

    pub fn remove_layout_execution_listener(&self, listener: &Rc<dyn ILayoutExecutionListener>) {
        let mut listeners = self.listeners.borrow_mut();
        if let Some(index) = listeners
            .iter()
            .position(|existing| Rc::ptr_eq(existing, listener))
        {
            listeners.remove(index);
        }
    }
}
