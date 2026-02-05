#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree<T> {
    pub node: T,
    pub children: Vec<Tree<T>>,
}

impl<T> Tree<T> {
    pub fn new(node: T) -> Self {
        Tree {
            node,
            children: Vec::new(),
        }
    }
}

impl<T: Clone + PartialEq> Tree<T> {
    pub fn add_child_to(&mut self, parent: &T, child: T) -> bool {
        if &self.node == parent {
            self.children.push(Tree::new(child));
            return true;
        }
        for subtree in &mut self.children {
            if subtree.add_child_to(parent, child.clone()) {
                return true;
            }
        }
        false
    }
}
