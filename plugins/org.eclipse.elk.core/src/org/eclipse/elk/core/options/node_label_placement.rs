use crate::org::eclipse::elk::core::util::{EnumSet, EnumSetType};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum NodeLabelPlacement {
    HLeft,
    HCenter,
    HRight,
    VTop,
    VCenter,
    VBottom,
    Inside,
    Outside,
    HPriority,
}

impl NodeLabelPlacement {
    pub fn value_of(index: usize) -> NodeLabelPlacement {
        Self::variants()[index]
    }

    pub fn fixed() -> EnumSet<NodeLabelPlacement> {
        EnumSet::none_of()
    }

    pub fn inside_top_left() -> EnumSet<NodeLabelPlacement> {
        EnumSet::of(&[
            NodeLabelPlacement::Inside,
            NodeLabelPlacement::VTop,
            NodeLabelPlacement::HLeft,
        ])
    }

    pub fn inside_top_center() -> EnumSet<NodeLabelPlacement> {
        EnumSet::of(&[
            NodeLabelPlacement::Inside,
            NodeLabelPlacement::VTop,
            NodeLabelPlacement::HCenter,
        ])
    }

    pub fn inside_top_right() -> EnumSet<NodeLabelPlacement> {
        EnumSet::of(&[
            NodeLabelPlacement::Inside,
            NodeLabelPlacement::VTop,
            NodeLabelPlacement::HRight,
        ])
    }

    pub fn inside_center() -> EnumSet<NodeLabelPlacement> {
        EnumSet::of(&[
            NodeLabelPlacement::Inside,
            NodeLabelPlacement::VCenter,
            NodeLabelPlacement::HCenter,
        ])
    }

    pub fn inside_bottom_left() -> EnumSet<NodeLabelPlacement> {
        EnumSet::of(&[
            NodeLabelPlacement::Inside,
            NodeLabelPlacement::VBottom,
            NodeLabelPlacement::HLeft,
        ])
    }

    pub fn inside_bottom_center() -> EnumSet<NodeLabelPlacement> {
        EnumSet::of(&[
            NodeLabelPlacement::Inside,
            NodeLabelPlacement::VBottom,
            NodeLabelPlacement::HCenter,
        ])
    }

    pub fn inside_bottom_right() -> EnumSet<NodeLabelPlacement> {
        EnumSet::of(&[
            NodeLabelPlacement::Inside,
            NodeLabelPlacement::VBottom,
            NodeLabelPlacement::HRight,
        ])
    }

    pub fn outside_top_left() -> EnumSet<NodeLabelPlacement> {
        EnumSet::of(&[
            NodeLabelPlacement::Outside,
            NodeLabelPlacement::VTop,
            NodeLabelPlacement::HLeft,
        ])
    }

    pub fn outside_top_center() -> EnumSet<NodeLabelPlacement> {
        EnumSet::of(&[
            NodeLabelPlacement::Outside,
            NodeLabelPlacement::VTop,
            NodeLabelPlacement::HCenter,
        ])
    }

    pub fn outside_top_right() -> EnumSet<NodeLabelPlacement> {
        EnumSet::of(&[
            NodeLabelPlacement::Outside,
            NodeLabelPlacement::VTop,
            NodeLabelPlacement::HRight,
        ])
    }

    pub fn outside_bottom_left() -> EnumSet<NodeLabelPlacement> {
        EnumSet::of(&[
            NodeLabelPlacement::Outside,
            NodeLabelPlacement::VBottom,
            NodeLabelPlacement::HLeft,
        ])
    }

    pub fn outside_bottom_center() -> EnumSet<NodeLabelPlacement> {
        EnumSet::of(&[
            NodeLabelPlacement::Outside,
            NodeLabelPlacement::VBottom,
            NodeLabelPlacement::HCenter,
        ])
    }

    pub fn outside_bottom_right() -> EnumSet<NodeLabelPlacement> {
        EnumSet::of(&[
            NodeLabelPlacement::Outside,
            NodeLabelPlacement::VBottom,
            NodeLabelPlacement::HRight,
        ])
    }

    pub fn is_valid(placement: &EnumSet<NodeLabelPlacement>) -> bool {
        let inside_outside = [
            NodeLabelPlacement::Inside,
            NodeLabelPlacement::Outside,
        ];
        if count_members(placement, &inside_outside) > 1 {
            return false;
        }

        let horizontal = [
            NodeLabelPlacement::HLeft,
            NodeLabelPlacement::HCenter,
            NodeLabelPlacement::HRight,
        ];
        if count_members(placement, &horizontal) > 1 {
            return false;
        }

        let vertical = [
            NodeLabelPlacement::VTop,
            NodeLabelPlacement::VCenter,
            NodeLabelPlacement::VBottom,
        ];
        if count_members(placement, &vertical) > 1 {
            return false;
        }

        true
    }
}

impl EnumSetType for NodeLabelPlacement {
    fn variants() -> &'static [Self] {
        static VARIANTS: [NodeLabelPlacement; 9] = [
            NodeLabelPlacement::HLeft,
            NodeLabelPlacement::HCenter,
            NodeLabelPlacement::HRight,
            NodeLabelPlacement::VTop,
            NodeLabelPlacement::VCenter,
            NodeLabelPlacement::VBottom,
            NodeLabelPlacement::Inside,
            NodeLabelPlacement::Outside,
            NodeLabelPlacement::HPriority,
        ];
        &VARIANTS
    }
}

fn count_members(set: &EnumSet<NodeLabelPlacement>, candidates: &[NodeLabelPlacement]) -> usize {
    candidates.iter().filter(|c| set.contains(c)).count()
}
