use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSetType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum SPOrEPhases {
    P1Structure,
    P2ProcessingOrder,
    P3Execution,
}

impl EnumSetType for SPOrEPhases {
    fn variants() -> &'static [Self] {
        static VARIANTS: [SPOrEPhases; 3] = [
            SPOrEPhases::P1Structure,
            SPOrEPhases::P2ProcessingOrder,
            SPOrEPhases::P3Execution,
        ];
        &VARIANTS
    }
}
