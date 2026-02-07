use super::CNodeRef;

pub trait ISpacingsHandler {
    fn get_horizontal_spacing(&self, c_node1: &CNodeRef, c_node2: &CNodeRef) -> f64;
    fn get_vertical_spacing(&self, c_node1: &CNodeRef, c_node2: &CNodeRef) -> f64;
}

#[derive(Default)]
pub struct DefaultSpacingsHandler;

impl ISpacingsHandler for DefaultSpacingsHandler {
    fn get_horizontal_spacing(&self, _c_node1: &CNodeRef, _c_node2: &CNodeRef) -> f64 {
        0.0
    }

    fn get_vertical_spacing(&self, _c_node1: &CNodeRef, _c_node2: &CNodeRef) -> f64 {
        0.0
    }
}

impl<FH, FV> ISpacingsHandler for (FH, FV)
where
    FH: Fn(&CNodeRef, &CNodeRef) -> f64,
    FV: Fn(&CNodeRef, &CNodeRef) -> f64,
{
    fn get_horizontal_spacing(&self, c_node1: &CNodeRef, c_node2: &CNodeRef) -> f64 {
        (self.0)(c_node1, c_node2)
    }

    fn get_vertical_spacing(&self, c_node1: &CNodeRef, c_node2: &CNodeRef) -> f64 {
        (self.1)(c_node1, c_node2)
    }
}
