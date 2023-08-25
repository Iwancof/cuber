use super::super::client::ClientState;
use super::super::IResult;

use std::io::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PacketId {
    // Handshaking
    FirstPacket,
    LegacyServerListPing,

    // Login
    LoginDisconnect,
    LoginStart,
    EncryptionRequest,
    ClientAuth,
    EncryptionResponse,
    ServerAuth,
    SetCompression,
    LoginSuccess,

    // Play
    BundleDelimiter,
    SpawnEntify,
    SpawnPlayer,
    PluginMessage,
    ChangeDifficulty,
    LoginPlay,
    FeatureFlags,
    PlayerAbilities,
    SetHeldItem,
    UpdateRecipes,
    ChunkDataAndUpdateLight,
    Respawn,
    SynchronizePlayerPosition,

    // Meta
    Unset,
}

impl PacketId {
    pub(crate) fn to_serv_from_i32(state: ClientState, v: i32) -> IResult<Self> {
        match state {
            ClientState::Handshaking => Self::to_serv_in_handshake(v),
            ClientState::Login => Self::to_serv_in_login(v),
            ClientState::Status => {
                todo!()
            }
            ClientState::Play => Self::to_serv_in_play(v),
            ClientState::Initial => Err(Error::new(
                ErrorKind::InvalidData,
                "Client state is incorrect",
            )),
        }
    }

    fn to_serv_in_handshake(v: i32) -> IResult<Self> {
        match v {
            0x0 => Ok(Self::FirstPacket),
            0xfe => Ok(Self::LegacyServerListPing),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                "Unknown packet id (handshaking)",
            )),
        }
    }

    fn to_serv_in_login(v: i32) -> IResult<Self> {
        match v {
            0x0 => Ok(Self::LoginStart),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                "Unknown packet id (login)",
            )),
        }
    }

    fn to_serv_in_play(v: i32) -> IResult<Self> {
        match v {
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                "Unknown packet id (play)",
            )),
        }
    }

    pub fn to_client_to_i32(self) -> i32 {
        match self {
            // Handshaking

            // Login
            Self::LoginSuccess => 0x2,

            // Play
            Self::BundleDelimiter => 0,
            Self::SpawnEntify => 0x01,
            Self::SpawnPlayer => 0x03,
            Self::PluginMessage => 0x17,
            Self::ChangeDifficulty => 0x0c,
            Self::LoginPlay => 0x28,
            Self::FeatureFlags => 0x6b,
            Self::PlayerAbilities => 0x34,
            Self::SetHeldItem => 0x4d,
            Self::UpdateRecipes => 0x6d,
            Self::ChunkDataAndUpdateLight => 0x24,
            Self::Respawn => 0x41,
            Self::SynchronizePlayerPosition => 0x3c,

            Self::Unset => {
                unreachable!("Unset")
            }

            _ => {
                unimplemented!()
            }
        }
    }
}
