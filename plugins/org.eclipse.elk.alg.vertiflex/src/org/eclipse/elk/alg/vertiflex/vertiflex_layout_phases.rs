use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSetType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum VertiFlexLayoutPhases {
    P1NodeYPlacement,
    P2NodeRelativePlacement,
    P3NodeAbsolutePlacement,
    P4EdgeRouting,
}

impl EnumSetType for VertiFlexLayoutPhases {
    fn variants() -> &'static [Self] {
        static VARIANTS: [VertiFlexLayoutPhases; 4] = [
            VertiFlexLayoutPhases::P1NodeYPlacement,
            VertiFlexLayoutPhases::P2NodeRelativePlacement,
            VertiFlexLayoutPhases::P3NodeAbsolutePlacement,
            VertiFlexLayoutPhases::P4EdgeRouting,
        ];
        &VARIANTS
    }
}
