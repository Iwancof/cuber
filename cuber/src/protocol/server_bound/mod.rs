use super::primitive::Array;
use super::CResult;
use super::{primitive::BoolConditional, primitive::VarInt, Decodable};
use deriver::Decodable;
use packet_id::sb_packet;
use std::io::Read;
use uuid::Uuid;

pub trait ServerBoundPacket: Decodable {
    const PACKET_ID: i32;
}

pub trait PacketCluster: Sized {
    fn parse_with_id<T: Read>(id: i32, reader: &mut T) -> CResult<Self>;
    fn parse<T: Read>(reader: &mut T) -> CResult<Self> {
        let id = VarInt::decode(reader)?.into();
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
            fn parse_with_id<T: std::io::Read>(id: i32, reader: &mut T) -> CResult<Self> {
                #[deny(unreachable_patterns)]
                match id {
                    $(
                        /*
                        $struct_ident::PACKET_ID => {
                            println!("found packet: {}", stringify!($struct_ident));
                            let decoded = $struct_ident::decode(reader)?;
                            Ok(Self::$struct_ident(decoded))
                        }
                        */
                        $struct_ident::PACKET_ID => Ok(Self::$struct_ident($struct_ident::decode(reader)?)),
                    )*
                    id => {
                        CResult::Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Unknown packet id: {}", id)).into())
                    }
                }
            }
        }
        impl $enum_ident {
            paste::paste! {
                $(
                    pub fn [<assume_ $snake_name>](self) -> CResult<$struct_ident> {
                        if let Self::$struct_ident(inner) = self {
                            Ok(inner)
                        } else {
                            CResult::Err(std::io::Error::new(std::io::ErrorKind::Other, format!("expect {} but found {:?}", stringify!($struct_ident), self)).into())
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
    fn decode<T: Read>(reader: &mut T) -> CResult<Self> {
        match VarInt::decode(reader)?.into() {
            1 => Ok(Self::Status),
            2 => Ok(Self::Login),
            unk => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unknown next state: {}", unk),
            )
            .into()),
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
            payload: u8,
        }
    }
}

define_server_bound_packets! {
    #[derive(Debug)]
    pub enum Status {
        #[sb_packet(0)]
        #[derive(Decodable, Debug)]
        status_request: pub struct StatusRequest {

        }

        #[sb_packet(1)]
        #[derive(Decodable, Debug)]
        ping_request: pub struct PingRequest {
            payload: i64,
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
            pub shared_secret: Array<VarInt, u8>,
            pub verify_token: Array<VarInt, u8>,
        }

        #[sb_packet(2)]
        #[derive(Decodable, Debug, PartialEq, Eq, Clone, Hash)]
        plugin_response: pub struct PluginResponse {
            pub message_id: VarInt,
            pub data: BoolConditional<Array<VarInt, u8>>,
        }
    }
}
