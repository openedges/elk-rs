use crate::org::eclipse::elk::core::util::{EnumSet, EnumSetType};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum ContentAlignment {
    VTop,
    VCenter,
    VBottom,
    HLeft,
    HCenter,
    HRight,
}

impl ContentAlignment {
    pub fn center_center() -> EnumSet<ContentAlignment> {
        EnumSet::of(&[ContentAlignment::VCenter, ContentAlignment::HCenter])
    }

    pub fn top_left() -> EnumSet<ContentAlignment> {
        EnumSet::of(&[ContentAlignment::VTop, ContentAlignment::HLeft])
    }

    pub fn bottom_right() -> EnumSet<ContentAlignment> {
        EnumSet::of(&[ContentAlignment::VBottom, ContentAlignment::HRight])
    }

    pub fn top_center() -> EnumSet<ContentAlignment> {
        EnumSet::of(&[ContentAlignment::VTop, ContentAlignment::HCenter])
    }
}

impl EnumSetType for ContentAlignment {
    fn variants() -> &'static [Self] {
        static VARIANTS: [ContentAlignment; 6] = [
            ContentAlignment::VTop,
            ContentAlignment::VCenter,
            ContentAlignment::VBottom,
            ContentAlignment::HLeft,
            ContentAlignment::HCenter,
            ContentAlignment::HRight,
        ];
        &VARIANTS
    }
}
