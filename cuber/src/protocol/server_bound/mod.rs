use super::primitive::{
    array::{Array, PacketInferredInBytes, VarIntLength},
    Identifier,
};
use super::{primitive::BoolConditional, primitive::VarInt, Decodable};
use deriver::Decodable;
use packet_id::sb_packet;
use std::io::Read;
use uuid::Uuid;

use anyhow::{Result, Context as _, bail};

pub trait ServerBoundPacket: Decodable {
    const PACKET_ID: i32;
}

pub trait PacketCluster: Sized {
    fn parse_with_id<T: Read>(id: i32, reader: &mut T) -> Result<Self>;
    fn parse<T: Read>(reader: &mut T) -> Result<Self> {
        let id = VarInt::decode(reader).context("Failed to decode packet id")?.into();
        Self::parse_with_id(id, reader)
    }
}

macro_rules! define_server_bound_packets {
    {
        $(#[$enum_meta: meta])*
        $enum_vis: vis enum $enum_ident: ident {
            $(
                $(#[$struct_meta: meta])*
                $snake_name: ident : $struct_vis: vis struct $struct_ident: ident {
                    $(
                        $(#[$member_meta: meta])* $member_vis: vis $member: ident : $member_type: ty,
                    )*
                }
            )*
        }
    } => {
        $(
            $(#[$struct_meta])*
            $struct_vis struct $struct_ident {
                $(
                    $(#[$member_meta])* $member_vis $member: $member_type,
                )*
            }
        )*

        $(#[$enum_meta])*
        $enum_vis enum $enum_ident {
            $(
                $struct_ident($struct_ident),
            )*
        }

        impl PacketCluster for $enum_ident {
            fn parse_with_id<T: std::io::Read>(id: i32, #[allow(unused)] reader: &mut T) -> Result<Self> {
                #[deny(unreachable_patterns)]
                match id {
                    $(
                        $struct_ident::PACKET_ID => Ok(Self::$struct_ident($struct_ident::decode(reader).with_context(|| format!("Failed to decode {}", stringify!($struct_ident)))?)),
                    )*
                    id => {
                        bail!("Unknown packet id: {}", id)
                    }
                }
            }
        }
        impl $enum_ident {
            paste::paste! {
                $(
                    pub fn [<assume_ $snake_name>](self) -> Result<$struct_ident> {
                        if let Self::$struct_ident(inner) = self {
                            Ok(inner)
                        } else {
                            bail!("expect {} but found {:?}", stringify!($struct_ident), self)
                        }
                    }
                    pub fn [<unwrap_ $snake_name>](self) -> $struct_ident {
                        self.[<assume_ $snake_name>]().unwrap()
                    }
                )*
            }
        }
    };
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum HandshakeNextState {
    Status,
    Login,
}

impl Decodable for HandshakeNextState {
    fn decode<T: Read>(reader: &mut T) -> Result<Self> {
        match VarInt::decode(reader).context("Failed to decode next state")?.into() {
            1 => Ok(Self::Status),
            2 => Ok(Self::Login),
            unknown => bail!("Unknown next state: {}", unknown),
        }
    }
}

define_server_bound_packets! {
    #[derive(Debug)]
    pub enum Handshaking {
        #[sb_packet(0)]
        #[derive(Decodable, Debug)]
        handshake: pub struct Handshake {
            pub protocol_version: VarInt,
            pub server_address: String,
            pub server_port: u16,
            pub next_state: HandshakeNextState,
        }

        #[sb_packet(0xfe)]
        #[derive(Decodable, Debug)]
        legacy_server_list_ping: pub struct LegacyServerListPing {
            pub payload: u8,
        }
    }
}

define_server_bound_packets! {
    #[derive(Debug)]
    pub enum Status {
        #[sb_packet(0)]
        #[derive(Decodable, Debug)]
        status_request: pub struct StatusRequest { }

        #[sb_packet(1)]
        #[derive(Decodable, Debug)]
        ping_request: pub struct PingRequest {
            pub payload: i64,
        }
    }
}

define_server_bound_packets! {
    #[derive(Debug)]
    pub enum Login {
        #[sb_packet(0)]
        #[derive(Decodable, Debug, PartialEq, Eq, Clone, Hash)]
        login_start: pub struct LoginStart {
            pub name: String,
            pub uuid: BoolConditional<Uuid>,
        }

        #[sb_packet(1)]
        #[derive(Decodable, Debug, PartialEq, Eq, Clone, Hash)]
        encryption_response: pub struct EncryptionResponse {
            pub shared_secret: Array<VarIntLength, u8>,
            pub verify_token: Array<VarIntLength, u8>,
        }

        #[sb_packet(2)]
        #[derive(Decodable, Debug, PartialEq, Eq, Clone, Hash)]
        plugin_response: pub struct PluginResponse {
            pub message_id: VarInt,
            pub data: BoolConditional<Array<PacketInferredInBytes, u8>>,
        }
    }
}

define_server_bound_packets! {
    #[derive(Debug)]
    pub enum Play {
        #[sb_packet(0x00)]
        #[derive(Decodable, Debug, PartialEq, Eq, Clone, Hash)]
        confirm_teleportation: pub struct ConfirmTeleportation {
            pub teleport_id: VarInt,
        }

        #[sb_packet(0x08)]
        #[derive(Decodable, Debug, PartialEq, Eq, Clone, Hash)]
        client_information: pub struct ClientInformation {
            pub locale: String,
            pub view_distance: i8,
            pub chat_mode: VarInt,
            pub chat_colors: bool,
            pub displayed_skin_parts: u8,
            pub main_hand: VarInt,
            pub enable_text_filtering: bool,
            pub allow_server_listings: bool,
        }

        #[sb_packet(0x0d)]
        #[derive(Decodable, Debug, PartialEq, Eq, Clone, Hash)]
        plugin_message: pub struct PlguinMessage {
            channel: Identifier,
            data: Array<PacketInferredInBytes, u8>,
        }

        #[sb_packet(0x14)]
        #[derive(Decodable, Debug, PartialEq, Clone)]
        set_player_position: pub struct SetPlayerPosition {
            pub x: f64,
            pub feet_y: f64,
            pub z: f64,
            pub on_ground: bool,
        }

        #[sb_packet(0x15)]
        #[derive(Decodable, Debug, PartialEq, Clone)]
        set_player_position_and_rotation: pub struct SetPlayerPositionAndRotation {
            pub x: f64,
            pub feet_y: f64,
            pub z: f64,
            pub yaw: f32,
            pub pitch: f32,
            pub on_ground: bool,
        }
    }
}
