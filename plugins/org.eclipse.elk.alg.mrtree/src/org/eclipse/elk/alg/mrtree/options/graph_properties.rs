#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum GraphProperties {
    SelfLoops,
}

impl org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSetType for GraphProperties {
    fn variants() -> &'static [Self] {
        static VARIANTS: [GraphProperties; 1] = [GraphProperties::SelfLoops];
        &VARIANTS
    }
}
