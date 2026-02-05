#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum NeatoModel {
    Shortpath,
    Circuit,
    Subset,
}

impl NeatoModel {
    pub fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "shortpath" => NeatoModel::Shortpath,
            "circuit" => NeatoModel::Circuit,
            "subset" => NeatoModel::Subset,
            _ => NeatoModel::Shortpath,
        }
    }

    pub fn literal(self) -> &'static str {
        match self {
            NeatoModel::Shortpath => "shortpath",
            NeatoModel::Circuit => "circuit",
            NeatoModel::Subset => "subset",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum OverlapMode {
    None,
    Scale,
    Scalexy,
    Prism,
    Compress,
}

impl OverlapMode {
    pub fn parse(value: &str) -> Self {
        let value = value.to_ascii_lowercase();
        if value == "true" {
            return OverlapMode::None;
        }
        match value.as_str() {
            "none" => OverlapMode::None,
            "scale" => OverlapMode::Scale,
            "scalexy" => OverlapMode::Scalexy,
            "prism" => OverlapMode::Prism,
            "compress" => OverlapMode::Compress,
            _ => OverlapMode::None,
        }
    }

    pub fn literal(self) -> &'static str {
        match self {
            OverlapMode::None => "true",
            OverlapMode::Scale => "scale",
            OverlapMode::Scalexy => "scalexy",
            OverlapMode::Prism => "prism",
            OverlapMode::Compress => "compress",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Command {
    Invalid,
    Dot,
    Neato,
    Twopi,
    Fdp,
    Circo,
}

impl Command {
    pub fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "dot" => Command::Dot,
            "neato" => Command::Neato,
            "twopi" => Command::Twopi,
            "fdp" => Command::Fdp,
            "circo" => Command::Circo,
            _ => Command::Invalid,
        }
    }

    pub fn literal(self) -> &'static str {
        match self {
            Command::Invalid => "invalid",
            Command::Dot => "dot",
            Command::Neato => "neato",
            Command::Twopi => "twopi",
            Command::Fdp => "fdp",
            Command::Circo => "circo",
        }
    }
}
