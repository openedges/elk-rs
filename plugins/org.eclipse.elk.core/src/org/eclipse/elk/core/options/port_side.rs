use std::sync::LazyLock;

use crate::org::eclipse::elk::core::options::direction::Direction;
use crate::org::eclipse::elk::core::util::{EnumSet, EnumSetType};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum PortSide {
    Undefined,
    North,
    East,
    South,
    West,
}

impl PortSide {
    pub fn right(self) -> PortSide {
        match self {
            PortSide::North => PortSide::East,
            PortSide::East => PortSide::South,
            PortSide::South => PortSide::West,
            PortSide::West => PortSide::North,
            PortSide::Undefined => PortSide::Undefined,
        }
    }

    pub fn left(self) -> PortSide {
        match self {
            PortSide::North => PortSide::West,
            PortSide::West => PortSide::South,
            PortSide::South => PortSide::East,
            PortSide::East => PortSide::North,
            PortSide::Undefined => PortSide::Undefined,
        }
    }

    pub fn opposed(self) -> PortSide {
        match self {
            PortSide::North => PortSide::South,
            PortSide::East => PortSide::West,
            PortSide::South => PortSide::North,
            PortSide::West => PortSide::East,
            PortSide::Undefined => PortSide::Undefined,
        }
    }

    pub fn are_adjacent(self, other: PortSide) -> bool {
        if self == PortSide::Undefined {
            return false;
        }
        self.left() == other || self.right() == other
    }

    pub fn from_direction(direction: Direction) -> PortSide {
        match direction {
            Direction::Up => PortSide::North,
            Direction::Right => PortSide::East,
            Direction::Down => PortSide::South,
            Direction::Left => PortSide::West,
            _ => PortSide::Undefined,
        }
    }

    pub fn is_vertical(side: PortSide) -> bool {
        matches!(side, PortSide::North | PortSide::South)
    }

    pub fn is_horizontal(side: PortSide) -> bool {
        matches!(side, PortSide::West | PortSide::East)
    }
}

impl EnumSetType for PortSide {
    fn variants() -> &'static [Self] {
        static VARIANTS: [PortSide; 5] = [
            PortSide::Undefined,
            PortSide::North,
            PortSide::East,
            PortSide::South,
            PortSide::West,
        ];
        &VARIANTS
    }
}

pub static SIDES_NONE: LazyLock<EnumSet<PortSide>> = LazyLock::new(EnumSet::none_of);
pub static SIDES_NORTH: LazyLock<EnumSet<PortSide>> =
    LazyLock::new(|| EnumSet::of(&[PortSide::North]));
pub static SIDES_EAST: LazyLock<EnumSet<PortSide>> =
    LazyLock::new(|| EnumSet::of(&[PortSide::East]));
pub static SIDES_SOUTH: LazyLock<EnumSet<PortSide>> =
    LazyLock::new(|| EnumSet::of(&[PortSide::South]));
pub static SIDES_WEST: LazyLock<EnumSet<PortSide>> =
    LazyLock::new(|| EnumSet::of(&[PortSide::West]));
pub static SIDES_NORTH_SOUTH: LazyLock<EnumSet<PortSide>> =
    LazyLock::new(|| EnumSet::of(&[PortSide::North, PortSide::South]));
pub static SIDES_EAST_WEST: LazyLock<EnumSet<PortSide>> =
    LazyLock::new(|| EnumSet::of(&[PortSide::East, PortSide::West]));
pub static SIDES_NORTH_WEST: LazyLock<EnumSet<PortSide>> =
    LazyLock::new(|| EnumSet::of(&[PortSide::North, PortSide::West]));
pub static SIDES_NORTH_EAST: LazyLock<EnumSet<PortSide>> =
    LazyLock::new(|| EnumSet::of(&[PortSide::North, PortSide::East]));
pub static SIDES_SOUTH_WEST: LazyLock<EnumSet<PortSide>> =
    LazyLock::new(|| EnumSet::of(&[PortSide::South, PortSide::West]));
pub static SIDES_EAST_SOUTH: LazyLock<EnumSet<PortSide>> =
    LazyLock::new(|| EnumSet::of(&[PortSide::East, PortSide::South]));
pub static SIDES_NORTH_EAST_WEST: LazyLock<EnumSet<PortSide>> =
    LazyLock::new(|| EnumSet::of(&[PortSide::North, PortSide::East, PortSide::West]));
pub static SIDES_EAST_SOUTH_WEST: LazyLock<EnumSet<PortSide>> =
    LazyLock::new(|| EnumSet::of(&[PortSide::East, PortSide::South, PortSide::West]));
pub static SIDES_NORTH_SOUTH_WEST: LazyLock<EnumSet<PortSide>> =
    LazyLock::new(|| EnumSet::of(&[PortSide::North, PortSide::South, PortSide::West]));
pub static SIDES_NORTH_EAST_SOUTH: LazyLock<EnumSet<PortSide>> =
    LazyLock::new(|| EnumSet::of(&[PortSide::North, PortSide::East, PortSide::South]));
pub static SIDES_NORTH_EAST_SOUTH_WEST: LazyLock<EnumSet<PortSide>> = LazyLock::new(|| {
    EnumSet::of(&[
        PortSide::North,
        PortSide::East,
        PortSide::South,
        PortSide::West,
    ])
});
