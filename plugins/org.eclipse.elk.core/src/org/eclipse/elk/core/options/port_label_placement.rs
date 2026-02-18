use crate::org::eclipse::elk::core::util::{EnumSet, EnumSetType};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum PortLabelPlacement {
    Outside,
    Inside,
    NextToPortIfPossible,
    AlwaysSameSide,
    AlwaysOtherSameSide,
    SpaceEfficient,
}

impl PortLabelPlacement {
    pub fn fixed() -> EnumSet<PortLabelPlacement> {
        EnumSet::none_of()
    }

    pub fn inside() -> EnumSet<PortLabelPlacement> {
        EnumSet::of(&[PortLabelPlacement::Inside])
    }

    pub fn outside() -> EnumSet<PortLabelPlacement> {
        EnumSet::of(&[PortLabelPlacement::Outside])
    }

    pub fn is_fixed(placement: &EnumSet<PortLabelPlacement>) -> bool {
        !placement.contains(&PortLabelPlacement::Inside)
            && !placement.contains(&PortLabelPlacement::Outside)
    }

    pub fn is_valid(placement: &EnumSet<PortLabelPlacement>) -> bool {
        let inside_outside = count_members(
            placement,
            &[PortLabelPlacement::Inside, PortLabelPlacement::Outside],
        );
        if inside_outside > 1 {
            return false;
        }

        let position = count_members(
            placement,
            &[
                PortLabelPlacement::AlwaysSameSide,
                PortLabelPlacement::AlwaysOtherSameSide,
                PortLabelPlacement::SpaceEfficient,
            ],
        );
        if position > 1 {
            return false;
        }

        true
    }
}

impl EnumSetType for PortLabelPlacement {
    fn variants() -> &'static [Self] {
        static VARIANTS: [PortLabelPlacement; 6] = [
            PortLabelPlacement::Outside,
            PortLabelPlacement::Inside,
            PortLabelPlacement::NextToPortIfPossible,
            PortLabelPlacement::AlwaysSameSide,
            PortLabelPlacement::AlwaysOtherSameSide,
            PortLabelPlacement::SpaceEfficient,
        ];
        &VARIANTS
    }
}

fn count_members(set: &EnumSet<PortLabelPlacement>, candidates: &[PortLabelPlacement]) -> usize {
    candidates
        .iter()
        .filter(|entry| set.contains(entry))
        .count()
}
