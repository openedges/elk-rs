use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum LabelSide {
    Unknown,
    Above,
    Below,
    Inline,
}

pub static LABEL_SIDE_PROPERTY: LazyLock<Property<LabelSide>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.labelSide", LabelSide::Unknown));

impl LabelSide {
    pub const LABEL_SIDE: &'static LazyLock<Property<LabelSide>> = &LABEL_SIDE_PROPERTY;

    pub fn opposite(self) -> LabelSide {
        match self {
            LabelSide::Above => LabelSide::Below,
            LabelSide::Below => LabelSide::Above,
            LabelSide::Inline => LabelSide::Inline,
            LabelSide::Unknown => LabelSide::Unknown,
        }
    }
}
