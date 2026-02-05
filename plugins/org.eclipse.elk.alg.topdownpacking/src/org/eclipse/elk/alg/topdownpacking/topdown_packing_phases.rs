use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSetType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum TopdownPackingPhases {
    P1NodeArrangement,
    P2WhitespaceElimination,
}

impl EnumSetType for TopdownPackingPhases {
    fn variants() -> &'static [Self] {
        static VARIANTS: [TopdownPackingPhases; 2] = [
            TopdownPackingPhases::P1NodeArrangement,
            TopdownPackingPhases::P2WhitespaceElimination,
        ];
        &VARIANTS
    }
}
