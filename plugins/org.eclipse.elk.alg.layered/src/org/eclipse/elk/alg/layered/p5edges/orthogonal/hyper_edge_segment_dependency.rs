use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment::HyperEdgeSegmentRef;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyType {
    Regular,
    Critical,
}

pub const CRITICAL_DEPENDENCY_WEIGHT: i32 = 1;

pub type HyperEdgeSegmentDependencyRef = Rc<RefCell<HyperEdgeSegmentDependency>>;

pub struct HyperEdgeSegmentDependency {
    dependency_type: DependencyType,
    source: Option<Weak<RefCell<crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment::HyperEdgeSegment>>>,
    target: Option<Weak<RefCell<crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment::HyperEdgeSegment>>>,
    weight: i32,
}

impl HyperEdgeSegmentDependency {
    fn new(dependency_type: DependencyType, weight: i32) -> HyperEdgeSegmentDependencyRef {
        Rc::new(RefCell::new(HyperEdgeSegmentDependency {
            dependency_type,
            source: None,
            target: None,
            weight,
        }))
    }

    pub fn create_and_add_regular(
        source: &HyperEdgeSegmentRef,
        target: &HyperEdgeSegmentRef,
        weight: i32,
    ) -> HyperEdgeSegmentDependencyRef {
        let dep = HyperEdgeSegmentDependency::new(DependencyType::Regular, weight);
        HyperEdgeSegmentDependency::set_source(&dep, Some(source.clone()));
        HyperEdgeSegmentDependency::set_target(&dep, Some(target.clone()));
        dep
    }

    pub fn create_and_add_critical(
        source: &HyperEdgeSegmentRef,
        target: &HyperEdgeSegmentRef,
    ) -> HyperEdgeSegmentDependencyRef {
        let dep = HyperEdgeSegmentDependency::new(DependencyType::Critical, CRITICAL_DEPENDENCY_WEIGHT);
        HyperEdgeSegmentDependency::set_source(&dep, Some(source.clone()));
        HyperEdgeSegmentDependency::set_target(&dep, Some(target.clone()));
        dep
    }

    pub fn remove(dep: &HyperEdgeSegmentDependencyRef) {
        HyperEdgeSegmentDependency::set_source(dep, None);
        HyperEdgeSegmentDependency::set_target(dep, None);
    }

    pub fn reverse(dep: &HyperEdgeSegmentDependencyRef) {
        let (old_source, old_target) = {
            let dep_guard = dep.borrow();
            (dep_guard.source(), dep_guard.target())
        };
        HyperEdgeSegmentDependency::set_source(dep, old_target);
        HyperEdgeSegmentDependency::set_target(dep, old_source);
    }

    pub fn dependency_type(&self) -> DependencyType {
        self.dependency_type
    }

    pub fn source(&self) -> Option<HyperEdgeSegmentRef> {
        self.source.as_ref().and_then(|source| source.upgrade())
    }

    pub fn target(&self) -> Option<HyperEdgeSegmentRef> {
        self.target.as_ref().and_then(|target| target.upgrade())
    }

    pub fn weight(&self) -> i32 {
        self.weight
    }

    fn set_source(dep: &HyperEdgeSegmentDependencyRef, new_source: Option<HyperEdgeSegmentRef>) {
        let old_source = dep.borrow().source();
        if let Some(old_source) = old_source {
            old_source
                .borrow_mut()
                .remove_outgoing_dependency(dep);
        }

        {
            let mut dep_guard = dep.borrow_mut();
            dep_guard.source = new_source.as_ref().map(Rc::downgrade);
        }

        if let Some(new_source) = new_source {
            new_source.borrow_mut().add_outgoing_dependency(dep.clone());
        }
    }

    fn set_target(dep: &HyperEdgeSegmentDependencyRef, new_target: Option<HyperEdgeSegmentRef>) {
        let old_target = dep.borrow().target();
        if let Some(old_target) = old_target {
            old_target
                .borrow_mut()
                .remove_incoming_dependency(dep);
        }

        {
            let mut dep_guard = dep.borrow_mut();
            dep_guard.target = new_target.as_ref().map(Rc::downgrade);
        }

        if let Some(new_target) = new_target {
            new_target.borrow_mut().add_incoming_dependency(dep.clone());
        }
    }
}
