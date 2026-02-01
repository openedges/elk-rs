use crate::org::eclipse::elk::core::util::{EnumSet, EnumSetType};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum SizeConstraint {
    Ports,
    PortLabels,
    NodeLabels,
    MinimumSize,
}

impl SizeConstraint {
    pub fn fixed() -> EnumSet<SizeConstraint> {
        EnumSet::none_of()
    }

    pub fn minimum_size() -> EnumSet<SizeConstraint> {
        EnumSet::of(&[SizeConstraint::MinimumSize])
    }

    pub fn minimum_size_with_ports() -> EnumSet<SizeConstraint> {
        EnumSet::of(&[SizeConstraint::Ports, SizeConstraint::MinimumSize])
    }

    pub fn free() -> EnumSet<SizeConstraint> {
        EnumSet::all_of()
    }
}

impl EnumSetType for SizeConstraint {
    fn variants() -> &'static [Self] {
        static VARIANTS: [SizeConstraint; 4] = [
            SizeConstraint::Ports,
            SizeConstraint::PortLabels,
            SizeConstraint::NodeLabels,
            SizeConstraint::MinimumSize,
        ];
        &VARIANTS
    }
}

