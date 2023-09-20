use super::{Decodable, Encodable};

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
