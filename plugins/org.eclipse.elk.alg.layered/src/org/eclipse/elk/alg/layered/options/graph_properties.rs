use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSetType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum GraphProperties {
    Comments,
    ExternalPorts,
    Hyperedges,
    Hypernodes,
    NonFreePorts,
    NorthSouthPorts,
    SelfLoops,
    CenterLabels,
    EndLabels,
    Partitions,
}

impl EnumSetType for GraphProperties {
    fn variants() -> &'static [Self] {
        static VARIANTS: [GraphProperties; 10] = [
            GraphProperties::Comments,
            GraphProperties::ExternalPorts,
            GraphProperties::Hyperedges,
            GraphProperties::Hypernodes,
            GraphProperties::NonFreePorts,
            GraphProperties::NorthSouthPorts,
            GraphProperties::SelfLoops,
            GraphProperties::CenterLabels,
            GraphProperties::EndLabels,
            GraphProperties::Partitions,
        ];
        &VARIANTS
    }
}
