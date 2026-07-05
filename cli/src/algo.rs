//! Construction selection: the `--algo` flag and its artifact tag / domain.

use crate::artifact;

#[derive(Clone, Copy, PartialEq)]
pub enum Algo {
    Brakedown,
    Ikat,
    Binius,
    Porphyry,
}

impl Algo {
    pub fn name(self) -> &'static str {
        match self {
            Algo::Brakedown => "brakedown",
            Algo::Ikat => "ikat",
            Algo::Binius => "binius",
            Algo::Porphyry => "porphyry",
        }
    }

    pub fn tag(self) -> u8 {
        match self {
            Algo::Brakedown => artifact::TAG_BRAKEDOWN,
            Algo::Ikat => artifact::TAG_IKAT,
            Algo::Binius => artifact::TAG_BINIUS,
            Algo::Porphyry => artifact::TAG_PORPHYRY,
        }
    }

    pub fn from_tag(tag: u8) -> Result<Self, String> {
        match tag {
            artifact::TAG_BRAKEDOWN => Ok(Algo::Brakedown),
            artifact::TAG_IKAT => Ok(Algo::Ikat),
            artifact::TAG_BINIUS => Ok(Algo::Binius),
            artifact::TAG_PORPHYRY => Ok(Algo::Porphyry),
            other => Err(format!("unknown construction tag {other} in artifact")),
        }
    }

    /// Fiat-Shamir domain — identical between open and verify (specs/cli.md §4).
    pub fn domain(self) -> &'static [u8] {
        match self {
            Algo::Brakedown => b"lens/brakedown/v0.1.0",
            Algo::Ikat => b"lens/ikat/v0.1.0",
            Algo::Binius => b"lens/binius/v0.1.0",
            Algo::Porphyry => b"lens/porphyry/v0.1.0",
        }
    }
}

pub fn parse_algo(s: &str) -> Result<Algo, String> {
    match s {
        "brakedown" => Ok(Algo::Brakedown),
        "ikat" => Ok(Algo::Ikat),
        "binius" => Ok(Algo::Binius),
        "porphyry" => Ok(Algo::Porphyry),
        other => Err(format!("unknown construction '{other}'")),
    }
}
