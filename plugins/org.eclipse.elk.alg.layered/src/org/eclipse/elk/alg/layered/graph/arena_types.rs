//! Typed index newtypes for the arena-based graph representation.
//!
//! Each index wraps a `u32` and uses `u32::MAX` as a sentinel (`NONE`) for "no element".
//! All types are `Copy` and zero-cost at runtime.

/// Index into node arrays of [`super::LArena`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u32);

impl NodeId {
    pub const NONE: Self = NodeId(u32::MAX);

    #[inline]
    pub fn is_none(self) -> bool {
        self.0 == u32::MAX
    }

    #[inline]
    pub fn idx(self) -> usize {
        self.0 as usize
    }
}

/// Index into port arrays of [`super::LArena`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PortId(pub u32);

impl PortId {
    pub const NONE: Self = PortId(u32::MAX);

    #[inline]
    pub fn is_none(self) -> bool {
        self.0 == u32::MAX
    }

    #[inline]
    pub fn idx(self) -> usize {
        self.0 as usize
    }
}

/// Index into edge arrays of [`super::LArena`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeId(pub u32);

impl EdgeId {
    pub const NONE: Self = EdgeId(u32::MAX);

    #[inline]
    pub fn is_none(self) -> bool {
        self.0 == u32::MAX
    }

    #[inline]
    pub fn idx(self) -> usize {
        self.0 as usize
    }
}

/// Index into label arrays of [`super::LArena`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LabelId(pub u32);

impl LabelId {
    pub const NONE: Self = LabelId(u32::MAX);

    #[inline]
    pub fn is_none(self) -> bool {
        self.0 == u32::MAX
    }

    #[inline]
    pub fn idx(self) -> usize {
        self.0 as usize
    }
}

/// Index into layer arrays of [`super::LArena`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LayerId(pub u32);

impl LayerId {
    pub const NONE: Self = LayerId(u32::MAX);

    #[inline]
    pub fn is_none(self) -> bool {
        self.0 == u32::MAX
    }

    #[inline]
    pub fn idx(self) -> usize {
        self.0 as usize
    }
}
