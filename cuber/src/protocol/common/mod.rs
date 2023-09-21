use deriver::{Decodable, Encodable};

use super::{primitive::Identifier, Decodable, Encodable};

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub enum State {
    Handshaking,
    Status,
    Login,
    Play,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Compression {
    Disabled,
    Handhsaking,
    Enabled,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Encryption {
    Disabled,
    Handshaking,
    Enabled,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum GameMode {
    Undefined,
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl Decodable for GameMode {
    fn decode<T: std::io::Read>(reader: &mut T) -> super::CResult<Self> {
        let raw = i8::decode(reader)?;
        match raw {
            -1 => Ok(GameMode::Undefined),
            0 => Ok(GameMode::Survival),
            1 => Ok(GameMode::Creative),
            2 => Ok(GameMode::Adventure),
            3 => Ok(GameMode::Spectator),
            id => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid game mode id: {}", id),
            )
            .into()),
        }
    }
}

impl Encodable for GameMode {
    fn encode<T: std::io::Write>(&self, writer: &mut T) -> usize {
        let raw: i8 = match self {
            GameMode::Undefined => -1,
            GameMode::Survival => 0,
            GameMode::Creative => 1,
            GameMode::Adventure => 2,
            GameMode::Spectator => 3,
        };
        raw.encode(writer)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Feature {
    Vanilla,
    Bundle,
    Other(Identifier),
}

impl Encodable for Feature {
    fn encode<T: std::io::Write>(&self, writer: &mut T) -> usize {
        let ident = match self {
            Self::Vanilla => "minecraft:vanilla".into(),
            Self::Bundle => "minecraft:bundle".into(),
            Self::Other(ident) => ident.clone(),
        };

        ident.encode(writer)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
    Other(u8),
}

impl Decodable for Difficulty {
    fn decode<T: std::io::Read>(reader: &mut T) -> super::CResult<Self> {
        let raw = u8::decode(reader)?;
        match raw {
            0 => Ok(Difficulty::Peaceful),
            1 => Ok(Difficulty::Easy),
            2 => Ok(Difficulty::Normal),
            3 => Ok(Difficulty::Hard),
            id => Ok(Difficulty::Other(id)),
        }
    }
}

impl Encodable for Difficulty {
    fn encode<T: std::io::Write>(&self, writer: &mut T) -> usize {
        let raw: u8 = match self {
            Difficulty::Peaceful => 0,
            Difficulty::Easy => 1,
            Difficulty::Normal => 2,
            Difficulty::Hard => 3,
            Difficulty::Other(id) => *id,
        };
        raw.encode(writer)
    }
}

bitflags::bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
    pub struct PlayerAbilitiesFlags: u8 {
        const INVULNERABLE = 0b0000_0001;
        const FLYING = 0b0000_0010;
        const ALLOW_FLYING = 0b0000_0100;
        const CREATIVE_MODE = 0b0000_1000;
    }
}

impl Encodable for PlayerAbilitiesFlags {
    fn encode<T: std::io::Write>(&self, writer: &mut T) -> usize {
        self.bits().encode(writer)
    }
}

impl Decodable for PlayerAbilitiesFlags {
    fn decode<T: std::io::Read>(reader: &mut T) -> super::CResult<Self> {
        let raw = u8::decode(reader)?;
        match Self::from_bits(raw) {
            Some(flags) => Ok(flags),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid player abilities flags: {}", raw),
            )
            .into()),
        }
    }
}
