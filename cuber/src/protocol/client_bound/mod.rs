use std::io::BufWriter;

use deriver::Encodable;
use nbt::Blob;
use packet_id::cb_packet;
use structstruck;
use uuid::Uuid;

use super::{
    common::{Difficulty, Feature, GameMode, PlayerAbilitiesFlags, SynchronizePlayerPositionFlags},
    primitive::{
        array::{Array, PacketInferredInBytes, VarIntLength, VarIntLengthInBytes},
        Angle, BoolConditional, Chat, Identifier, Position, Todo, VarInt,
    },
    BuiltPacket, Encodable, State,
};

pub trait ClientBoundPacket: Encodable {
    const PACKET_ID: i32;
    const VALID_STATE: State;

    fn verify(&self, current_state: State) {
        assert_eq!(current_state, Self::VALID_STATE);
    }
    fn to_bytes(&self) -> Box<[u8]> {
        let mut buf: BufWriter<Vec<u8>> = BufWriter::new(Vec::new());
        VarInt(Self::PACKET_ID).encode(&mut buf);
        self.encode(&mut buf);

        buf.into_inner().unwrap().into_boxed_slice()
    }
    fn to_packet(&self) -> BuiltPacket {
        BuiltPacket {
            buf: self.to_bytes(),
        }
    }
}

#[cb_packet(State::Status, 0)]
#[derive(Encodable, Debug, PartialEq, Eq, Clone)]
pub struct StatusResponse {
    pub json_response: String, // replace with Json object.
}

#[cb_packet(State::Login, 0)]
#[derive(Encodable, Debug, PartialEq, Eq, Clone)]
pub struct Disconnect {
    pub chat: Chat,
}

#[cb_packet(State::Login, 1)]
#[derive(Encodable, Debug, PartialEq, Eq, Clone)]
pub struct EncryptionRequest {
    pub server_id: String,
    pub public_key: Array<VarIntLength, u8>,
    pub verify_token: Array<VarIntLength, u8>,
}

structstruck::strike! {
    #[cb_packet(State::Login, 0x02)]
    #[derive(Encodable, Debug, PartialEq, Eq, Clone)]
    pub struct LoginSuccess {
        pub uuid: Uuid,
        pub user_name: String,
        pub property: Array<VarIntLength, #[derive(Encodable, Debug, PartialEq, Eq, Clone)] pub struct LoginSuccessProperty {
            pub name: String,
            pub value: String,
            pub signature: BoolConditional<String>,
        }>,
    }
}

#[cb_packet(State::Login, 0x03)]
#[derive(Encodable, Debug, PartialEq, Eq, Clone)]
pub struct SetCompression {
    pub threshold: VarInt,
}

#[cb_packet(State::Login, 0x04)]
#[derive(Encodable, Debug, PartialEq, Eq, Clone)]
pub struct PluginRequest {
    pub message_id: VarInt,
    pub channel: Identifier,
    pub data: Array<PacketInferredInBytes, u8>,
}

#[cb_packet(State::Play, 0x01)]
#[derive(Encodable, Debug, PartialEq, Clone, Copy)]
pub struct SpawnEntity {
    pub entity_id: VarInt,
    pub entity_uuid: Uuid,
    pub mob_type: VarInt,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub pitch: Angle,
    pub yaw: Angle,
    pub head_yaw: Angle,
    pub data: VarInt,
    pub velocity_x: i16,
    pub velocity_y: i16,
    pub velocity_z: i16,
}

#[cb_packet(State::Play, 0x0C)]
#[derive(Encodable, Debug, PartialEq, Eq, Clone)]
pub struct ChangeDifficulty {
    pub new_difficulty: Difficulty,
    pub difficulty_locked: bool,
}

#[cb_packet(State::Play, 0x17)]
#[derive(Encodable, Debug, PartialEq, Eq, Clone)]
pub struct PluginMessage {
    pub channel: Identifier,
    pub data: Array<PacketInferredInBytes, u8>,
}

structstruck::strike! {
    #[cb_packet(State::Play, 0x24)]
    #[derive(Encodable, Debug, PartialEq, Clone)]
    pub struct ChunkDataAndUpdateLight {
        chunk_x: i32,
        chunk_z: i32,
        height_maps: nbt::Blob,
        chunk_data: Array<VarIntLengthInBytes, #[derive(Encodable, Debug, PartialEq, Clone)] pub struct ChunkSection {
            block_count: i16,
            block_states: #[derive(Encodable, Debug, PartialEq, Clone)] pub struct PalettedContainer {
                bits_per_entry: u8,
                palette: Todo,
                data_array: Array<VarIntLength, i64>,
            },
            biomes: PalettedContainer,
        }>,
        block_entities: Array<VarIntLength, #[derive(Encodable, Debug, PartialEq, Clone)] pub struct BlockEntity {
            xy: Todo,
            y: u16,
            be_type: VarInt,
            data: nbt::Blob,
        }>,
        sky_light_mask: Todo,
        block_light_mask: Todo,
        empty_sky_light_mask: Todo,
        empty_block_light_mask: Todo,
        sky_lights_array: Array<VarIntLength, Array<VarIntLength, Todo>>,
        block_lights_array: Array<VarIntLength, Array<VarIntLength, Todo>>,
    }
}

structstruck::strike! {
    #[cb_packet(State::Play, 0x28)]
    #[derive(Encodable, Debug, PartialEq, Clone)]
    pub struct LoginPlay {
        pub(crate) entity_id: i32, // TODO: replace with Entity structure
        pub(crate) is_hardcore: bool,
        pub(crate) game_mode: GameMode,
        pub(crate) previous_game_mode: GameMode,
        pub(crate) dimension_names: Array<VarIntLength, Identifier>,
        pub(crate) registry_codec: Blob,
        pub(crate) dimension_type: Identifier,
        pub(crate) dimension_name: Identifier,
        pub(crate) hashed_seed: u64,
        pub(crate) max_players: VarInt,
        pub(crate) view_distance: VarInt,
        pub(crate) simulation_distance: VarInt,
        pub(crate) reduce_debug_info: bool,
        pub(crate) enable_respawn_screen: bool,
        pub(crate) is_debug: bool,
        pub(crate) is_flat: bool,
        pub(crate) death_location: BoolConditional<#[derive(Encodable, Debug, PartialEq, Eq, Clone)] pub struct DeathLocation {
            pub(crate) dimension_name: Identifier,
            pub(crate) location: Position,
        }>,
        pub(crate) portal_cooldown: VarInt,
    }
}

#[cb_packet(State::Play, 0x34)]
#[derive(Encodable, Debug, PartialEq, Clone)]
pub struct PlayerAbilities {
    pub flags: PlayerAbilitiesFlags,
    pub flying_speed: f32,
    pub field_of_view_modifier: f32,
}

structstruck::strike! {
    #[cb_packet(State::Play, 0x3c)]
    #[derive(Encodable, Debug, PartialEq, Clone)]
    pub struct SynchronizePlayerPosition {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
        pub flags: SynchronizePlayerPositionFlags,
        pub teleport_id: VarInt,
    }
}

#[cb_packet(State::Play, 0x4d)]
#[derive(Encodable, Debug, PartialEq, Clone)]
pub struct SetHeldItem {
    pub slot: u8,
}

#[cb_packet(State::Play, 0x6b)]
#[derive(Encodable, Debug, PartialEq, Eq, Clone)]
pub struct FeatureFlags {
    pub features: Array<VarIntLength, Feature>,
}
