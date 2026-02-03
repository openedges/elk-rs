use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSetType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum LayeredPhases {
    P1CycleBreaking,
    P2Layering,
    P3NodeOrdering,
    P4NodePlacement,
    P5EdgeRouting,
}

impl EnumSetType for LayeredPhases {
    fn variants() -> &'static [Self] {
        static VARIANTS: [LayeredPhases; 5] = [
            LayeredPhases::P1CycleBreaking,
            LayeredPhases::P2Layering,
            LayeredPhases::P3NodeOrdering,
            LayeredPhases::P4NodePlacement,
            LayeredPhases::P5EdgeRouting,
        ];
        &VARIANTS
    }
}
