use std::cell::RefCell;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::service::CompoundGraphElementVisitor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, IGraphElementVisitor};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

struct CountingVisitor {
    counter: Rc<RefCell<i32>>,
}

impl IGraphElementVisitor for CountingVisitor {
    fn visit(&mut self, _element: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef) {
        *self.counter.borrow_mut() += 1;
    }
}

#[test]
fn compound_visitor_applies_in_order() {
    let root = ElkGraphUtil::create_graph();
    let _child = ElkGraphUtil::create_node(Some(root.clone()));

    let counter_a = Rc::new(RefCell::new(0));
    let counter_b = Rc::new(RefCell::new(0));

    let visitor_a = CountingVisitor {
        counter: counter_a.clone(),
    };
    let visitor_b = CountingVisitor {
        counter: counter_b.clone(),
    };

    let mut compound = CompoundGraphElementVisitor::new(vec![
        Box::new(visitor_a),
        Box::new(visitor_b),
    ]);

    let mut visitors: Vec<&mut dyn IGraphElementVisitor> = vec![&mut compound];
    ElkUtil::apply_visitors(&root, &mut visitors);

    assert_eq!(*counter_a.borrow(), 2);
    assert_eq!(*counter_b.borrow(), 2);
}

#[test]
fn compound_visitor_full_graph_first() {
    let root = ElkGraphUtil::create_graph();
    let _child = ElkGraphUtil::create_node(Some(root.clone()));

    let counter = Rc::new(RefCell::new(0));
    let visitor = CountingVisitor {
        counter: counter.clone(),
    };

    let mut compound = CompoundGraphElementVisitor::new_with_mode(true, vec![Box::new(visitor)]);

    let mut visitors: Vec<&mut dyn IGraphElementVisitor> = vec![&mut compound];
    ElkUtil::apply_visitors(&root, &mut visitors);

    assert_eq!(*counter.borrow(), 2);
}
