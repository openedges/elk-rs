use std::collections::{BTreeSet, HashSet, LinkedList};
use std::sync::OnceLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

mod layout_algorithm_data;

pub use layout_algorithm_data::LayoutAlgorithmData;

use crate::org::eclipse::elk::core::math::{ElkMargin, ElkPadding, KVector, KVectorChain};
use crate::org::eclipse::elk::core::options::{
    Alignment, ContentAlignment, Direction, EdgeLabelPlacement, EdgeRouting, NodeLabelPlacement,
    PortConstraints, PortLabelPlacement, PortSide, SizeConstraint, SizeOptions,
};
use crate::org::eclipse::elk::core::util::{EnumSet, IndividualSpacings, LinkedHashSet};

pub struct LayoutMetaDataService;

static INSTANCE: OnceLock<LayoutMetaDataService> = OnceLock::new();

impl LayoutMetaDataService {
    pub fn get_instance() -> &'static LayoutMetaDataService {
        INSTANCE.get_or_init(|| {
            LayoutMetaDataService::init_elk_reflect();
            LayoutMetaDataService
        })
    }

    pub fn unload() {
        // OnceLock cannot be reset safely without unsafe; keep as no-op for now.
    }

    pub fn init_elk_reflect() {
        ElkReflect::register(Some(KVector::new), Some(|v: &KVector| *v));
        ElkReflect::register(Some(KVectorChain::new), Some(|vc: &KVectorChain| vc.clone()));
        ElkReflect::register(Some(ElkMargin::new), Some(|m: &ElkMargin| m.clone()));
        ElkReflect::register(Some(ElkPadding::new), Some(|p: &ElkPadding| p.clone()));
        ElkReflect::register(Some(IndividualSpacings::new), Some(|s: &IndividualSpacings| {
            IndividualSpacings::from_other(s)
        }));

        ElkReflect::register(Some(|| 0_i32), Some(|v: &i32| *v));
        ElkReflect::register(Some(|| 0_f32), Some(|v: &f32| *v));
        ElkReflect::register(Some(|| 0_f64), Some(|v: &f64| *v));
        ElkReflect::register(Some(|| false), Some(|v: &bool| *v));
        ElkReflect::register(Some(String::new), Some(|v: &String| v.clone()));
        ElkReflect::register(Some(|| Alignment::Automatic), Some(|v: &Alignment| *v));
        ElkReflect::register_default_clone::<LayoutAlgorithmData>();
        ElkReflect::register(Some(|| PortSide::Undefined), Some(|v: &PortSide| *v));
        ElkReflect::register(Some(|| Direction::Undefined), Some(|v: &Direction| *v));
        ElkReflect::register(Some(|| EdgeRouting::Undefined), Some(|v: &EdgeRouting| *v));
        ElkReflect::register(Some(|| PortConstraints::Undefined), Some(|v: &PortConstraints| *v));
        ElkReflect::register(Some(|| EdgeLabelPlacement::Center), Some(|v: &EdgeLabelPlacement| *v));

        ElkReflect::register(Some(Vec::<KVector>::new), Some(|v: &Vec<KVector>| v.clone()));
        ElkReflect::register(Some(LinkedList::<KVector>::new), Some(|v: &LinkedList<KVector>| {
            v.clone()
        }));
        ElkReflect::register(Some(HashSet::<KVector>::new), Some(|v: &HashSet<KVector>| v.clone()));
        ElkReflect::register(Some(LinkedHashSet::<KVector>::new), Some(|v: &LinkedHashSet<KVector>| {
            v.clone()
        }));
        ElkReflect::register(Some(BTreeSet::<i32>::new), Some(|v: &BTreeSet<i32>| v.clone()));
        ElkReflect::register(Some(EnumSet::<SizeConstraint>::none_of), Some(|v: &EnumSet<SizeConstraint>| {
            v.clone()
        }));
        ElkReflect::register(
            Some(EnumSet::<ContentAlignment>::none_of),
            Some(|v: &EnumSet<ContentAlignment>| v.clone()),
        );
        ElkReflect::register(
            Some(EnumSet::<SizeOptions>::none_of),
            Some(|v: &EnumSet<SizeOptions>| v.clone()),
        );
        ElkReflect::register(
            Some(EnumSet::<NodeLabelPlacement>::none_of),
            Some(|v: &EnumSet<NodeLabelPlacement>| v.clone()),
        );
        ElkReflect::register(
            Some(EnumSet::<PortLabelPlacement>::none_of),
            Some(|v: &EnumSet<PortLabelPlacement>| v.clone()),
        );
    }
}
