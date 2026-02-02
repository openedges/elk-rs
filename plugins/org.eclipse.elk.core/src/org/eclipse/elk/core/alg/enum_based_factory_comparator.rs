use std::cmp::Ordering;
use std::sync::Arc;

use super::i_layout_processor_factory::ILayoutProcessorFactory;

pub struct EnumBasedFactoryComparator;

impl EnumBasedFactoryComparator {
    pub fn compare<G>(
        &self,
        factory1: &Arc<dyn ILayoutProcessorFactory<G>>,
        factory2: &Arc<dyn ILayoutProcessorFactory<G>>,
    ) -> Ordering {
        let type1 = factory1.enum_type_id();
        let type2 = factory2.enum_type_id();
        let ord1 = factory1.enum_ordinal();
        let ord2 = factory2.enum_ordinal();

        if type1.is_none()
            || type2.is_none()
            || ord1.is_none()
            || ord2.is_none()
            || type1 != type2
        {
            panic!(
                "This comparator can only compare enumeration constants that are part of the same enumeration."
            );
        }

        ord1.unwrap().cmp(&ord2.unwrap())
    }
}
