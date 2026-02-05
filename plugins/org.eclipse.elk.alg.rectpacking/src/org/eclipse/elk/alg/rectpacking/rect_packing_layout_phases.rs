use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSetType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum RectPackingLayoutPhases {
    P1WidthApproximation,
    P2Packing,
    P3WhitespaceElimination,
}

impl EnumSetType for RectPackingLayoutPhases {
    fn variants() -> &'static [Self] {
        static VARIANTS: [RectPackingLayoutPhases; 3] = [
            RectPackingLayoutPhases::P1WidthApproximation,
            RectPackingLayoutPhases::P2Packing,
            RectPackingLayoutPhases::P3WhitespaceElimination,
        ];
        &VARIANTS
    }
}
