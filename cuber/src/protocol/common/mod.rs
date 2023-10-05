use std::hash::Hash;

use super::{
    primitive::{
        array::{Array, VarIntLengthInBytes},
        Identifier,
    },
    Decodable, Encodable,
};
use deriver::{Decodable, Encodable};

use anyhow::{bail, Context as _, Result};

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
    fn decode<T: std::io::Read>(reader: &mut T) -> Result<Self> {
        let raw = i8::decode(reader).context("Failed to decode game mode")?;
        match raw {
            -1 => Ok(GameMode::Undefined),
            0 => Ok(GameMode::Survival),
            1 => Ok(GameMode::Creative),
            2 => Ok(GameMode::Adventure),
            3 => Ok(GameMode::Spectator),
            id => bail!("Invalid game mode: {}", id),
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
    fn decode<T: std::io::Read>(reader: &mut T) -> Result<Self> {
        let raw = u8::decode(reader).context("Failed to decode difficulty")?;
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
    fn decode<T: std::io::Read>(reader: &mut T) -> Result<Self> {
        let raw = u8::decode(reader).context("Failed to decode player abilities flags")?;
        match Self::from_bits(raw) {
            Some(flags) => Ok(flags),
            None => bail!("Invalid player abilities flags: {}", raw),
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
    pub struct SynchronizePlayerPositionFlags: u8 {
        const X     = 0b0000_0001;
        const Y     = 0b0000_0010;
        const Z     = 0b0000_0100;
        const Y_ROT = 0b0000_1000;
        const X_ROP = 0b0001_0000;
    }
}

impl Encodable for SynchronizePlayerPositionFlags {
    fn encode<T: std::io::Write>(&self, writer: &mut T) -> usize {
        self.bits().encode(writer)
    }
}

impl Decodable for SynchronizePlayerPositionFlags {
    fn decode<T: std::io::Read>(reader: &mut T) -> Result<Self> {
        let raw = u8::decode(reader).context("Failed to decode synchronize player position flags")?;
        match Self::from_bits(raw) {
            Some(flags) => Ok(flags),
            None => bail!("Invalid synchronize player position flags: {}", raw),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InChunkOffset {
    x: i32,
    z: i32,
}

impl InChunkOffset {
    pub fn pack(self) -> i8 {
        let x = self.x & 0x0f;
        let z = self.z & 0x0f;
        ((x << 4) | z) as i8
    }
}

impl PartialEq for InChunkOffset {
    fn eq(&self, other: &Self) -> bool {
        self.pack() == other.pack()
    }
}

impl Eq for InChunkOffset {}
impl Hash for InChunkOffset {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pack().hash(state)
    }
}

impl Encodable for InChunkOffset {
    fn encode<T: std::io::Write>(&self, writer: &mut T) -> usize {
        self.pack().encode(writer)
    }
}

impl Decodable for InChunkOffset {
    fn decode<R: std::io::Read>(reader: &mut R) -> Result<Self> {
        let raw = i8::decode(reader).context("Failed to decode in-chunk offset")?;
        let x = (raw >> 4) & 0x0f;
        let z = raw & 0x0f;
        Ok(Self {
            x: x as i32,
            z: z as i32,
        })
    }
}

#[derive(Decodable, Encodable, Debug, PartialEq, Eq, Clone, Hash)]
pub struct SkyLightArray {
    array: Array<VarIntLengthInBytes, u8>,
}

impl SkyLightArray {
    pub fn to_index(x: u32, y: u32, z: u32) -> (usize, bool) {
        assert!(x < 16);
        assert!(y < 16);
        assert!(z < 16);

        let index = (y << 8) | (z << 4) | x;
        let is_upper = index & 1 == 1;

        (index as usize >> 1, is_upper)
    }
    pub fn get(&self, x: u32, y: u32, z: u32) -> u8 {
        let (index, is_upper) = Self::to_index(x, y, z);
        let byte = self.array.inner[index];
        if is_upper {
            byte >> 4
        } else {
            byte & 0x0f
        }
    }
}
