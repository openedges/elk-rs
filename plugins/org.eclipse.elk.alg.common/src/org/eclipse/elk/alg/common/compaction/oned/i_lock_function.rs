use org_eclipse_elk_core::org::eclipse::elk::core::options::Direction;

use super::CNodeRef;

pub trait ILockFunction {
    fn is_locked(&self, node: &CNodeRef, direction: Direction) -> bool;
}

impl<F> ILockFunction for F
where
    F: Fn(&CNodeRef, Direction) -> bool,
{
    fn is_locked(&self, node: &CNodeRef, direction: Direction) -> bool {
        self(node, direction)
    }
}
