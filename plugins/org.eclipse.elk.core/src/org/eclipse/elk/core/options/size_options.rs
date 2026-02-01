use crate::org::eclipse::elk::core::util::EnumSetType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum SizeOptions {
    DefaultMinimumSize,
    MinimumSizeAccountsForPadding,
    ComputePadding,
    OutsideNodeLabelsOverhang,
    PortsOverhang,
    UniformPortSpacing,
    SpaceEfficientPortLabels,
    ForceTabularNodeLabels,
    Asymmetrical,
}

impl SizeOptions {
    pub fn value_of(index: usize) -> SizeOptions {
        Self::variants()[index]
    }
}

impl EnumSetType for SizeOptions {
    fn variants() -> &'static [Self] {
        static VARIANTS: [SizeOptions; 9] = [
            SizeOptions::DefaultMinimumSize,
            SizeOptions::MinimumSizeAccountsForPadding,
            SizeOptions::ComputePadding,
            SizeOptions::OutsideNodeLabelsOverhang,
            SizeOptions::PortsOverhang,
            SizeOptions::UniformPortSpacing,
            SizeOptions::SpaceEfficientPortLabels,
            SizeOptions::ForceTabularNodeLabels,
            SizeOptions::Asymmetrical,
        ];
        &VARIANTS
    }
}
