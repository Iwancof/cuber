pub mod packet_id;

use std::io::Write;

use packet_id::PacketId;

use super::client::ClientState;
use super::data_types::*;
use super::IResult;
use byteorder::WriteBytesExt;
use byteorder::{BigEndian, ReadBytesExt};

use std::io::Cursor;
use std::io::Read;

#[derive(ToPrimitive, FromPrimitive, Debug, PartialEq, Eq)]
pub enum HandshakeNextState {
    Status = 1,
    Login = 2,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FeatureFlags {
    Vanilla,
    Bundle
}

impl FeatureFlags {
    fn to_ident(self) -> &'static str {
        match self {
            Self::Vanilla => "minecraft:vanilla",
            Self::Bundle => "minecraft:bundle",
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct EntityId {
    pub id: i32,
}


use bitflags::bitflags;
bitflags! {
    pub struct PlayerAbilitiesFlags: u8 {
        const InVulnerable      = 0b00000001;
        const Flying            = 0b00000010;
        const AllowFlying       = 0b00000100;
        const CreativeMode      = 0b00001000;
    }
}

#[derive(Debug)]
pub struct Recipes();

#[derive(Debug)]
pub struct PacketParser {
    state: ClientState,
    packet_data: std::io::Cursor<Vec<u8>>,
}

impl PacketParser {
    pub fn from_state_vec(state: ClientState, v: Vec<u8>) -> Self {
        Self {
            state,
            packet_data: Cursor::new(v),
        }
    }
    pub(crate) fn read_unsigned_short(&mut self) -> IResult<u16> {
        self.packet_data.read_u16::<byteorder::BigEndian>()
    }
    pub(crate) fn read_var_int(&mut self) -> IResult<i32> {
        let (_read, data) = read_var_int(&mut self.packet_data)?;
        Ok(data)
    }
    pub(crate) fn read_string(&mut self) -> IResult<String> {
        let size = self.read_var_int()? as usize;
        let mut string_buf = vec![0u8; size];
        self.packet_data.read_exact(&mut string_buf)?;

        Ok(String::from_utf8(string_buf)
            .map_err(|_| (std::io::Error::new(std::io::ErrorKind::InvalidData, "Unicode error")))?)
    }
    pub(crate) fn read_uuid(&mut self) -> IResult<uuid::Uuid> {
        let raw = self.packet_data.read_u128::<BigEndian>()?;
        Ok(uuid::Uuid::from_u128(raw))
    }
    pub(crate) fn read_boolean(&mut self) -> IResult<bool> {
        let raw = self.packet_data.read_u8()?;
        if raw == 1 {
            Ok(true)
        } else if raw == 0 {
            Ok(false)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unknown boolean value: {}", raw),
            ))
        }
    }

    pub fn read_packet_id(&mut self) -> IResult<PacketId> {
        PacketId::to_serv_from_i32(self.state, self.read_var_int()?)
    }
    pub fn read_version(&mut self) -> IResult<i32> {
        let version = self.read_var_int()?;

        match version {
            763 => {
                println!("Minecraft client version is 1.20.1");
            }
            _ => {
                println!("Unsupported minecraft version");
            }
        }

        Ok(version)
    }
    pub fn get_next_state(&mut self) -> IResult<HandshakeNextState> {
        use num_traits::FromPrimitive;

        match FromPrimitive::from_i32(self.read_var_int()?) {
            None => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unknown handshake next state",
            )),
            Some(val) => Ok(val),
        }
    }

    /// Return
    /// Ok((version, hostname, port));
    pub fn handle_handshake(&mut self) -> IResult<(i32, String, u16)> {
        let id = self.read_packet_id()?;
        assert_eq!(id, PacketId::FirstPacket);

        let version = self.read_version()?;
        assert_eq!(version, 763);
        println!("[Handshaking] The version is 1.20.1");

        let host = self.read_string()?;
        let port = self.read_unsigned_short()?;
        println!("Attempt to connect to {}:{}", host, port);

        let state = self.get_next_state()?;
        assert_eq!(state, HandshakeNextState::Login);

        Ok((version, host, port))
    }

    pub fn handle_login_start(&mut self) -> IResult<(String, Option<uuid::Uuid>)> {
        let id = self.read_packet_id()?;
        assert_eq!(id, PacketId::LoginStart);

        let name = self.read_string()?;
        println!("name = {}", name);
        if self.read_boolean()? {
            Ok((name, Some(self.read_uuid()?)))
        } else {
            Ok((name, None))
        }
    }
}

#[derive(Debug, Clone)]
pub struct PacketBuilder {
    packet_id: PacketId,
    built_data: Cursor<Vec<u8>>,
}

impl PacketBuilder {
    pub fn new() -> Self {
        Self {
            packet_id: PacketId::Unset,
            built_data: Cursor::new(Vec::new()),
        }
    }

    pub(crate) fn to_packet_bytes(self) -> Vec<u8> {
        assert_ne!(self.packet_id, PacketId::Unset);

        let mut built_data = self.built_data.into_inner();

        built_data.insert(0, self.packet_id.to_client_to_i32() as u8); // TODO (Packet ID は必ず u8
                                                                       // で表される？)

        let length = built_data.len() as i32;
        let mut result = build_var_int(length);

        result.append(&mut built_data);
        result
    }
    pub(crate) fn set_packet_id(&mut self, id: PacketId) {
        self.packet_id = id;
    }
    pub(crate) fn write_var_int(&mut self, val: i32) -> IResult<()> {
        let data = build_var_int(val);
        self.built_data.write_all(&data)
    }
    pub(crate) fn write_string(&mut self, s: String) -> IResult<()> {
        let string_length = s.len();
        self.write_var_int(string_length as i32)?;
        self.built_data.write_all(&s.as_bytes()[..string_length])
    }
    pub(crate) fn write_uuid(&mut self, uuid: uuid::Uuid) -> IResult<()> {
        self.built_data.write_all(uuid.as_bytes())
    }
    pub(crate) fn write_unsigned_byte(&mut self, byte: u8) -> IResult<()> {
        self.built_data.write_u8(byte)
    }
    pub(crate) fn write_byte(&mut self, byte: i8) -> IResult<()> {
        self.built_data.write_i8(byte)
    }
    pub(crate) fn write_short(&mut self, val: i16) -> IResult<()> {
        self.built_data.write_i16::<BigEndian>(val)
    }
    pub(crate) fn write_int(&mut self, val: i32) -> IResult<()> {
        self.built_data.write_i32::<BigEndian>(val)
    }
    pub(crate) fn write_long(&mut self, v: i64) -> IResult<()> {
        self.built_data.write_i64::<BigEndian>(v)
    }
    pub(crate) fn write_boolean(&mut self, b: bool) -> IResult<()> {
        if b {
            self.write_unsigned_byte(1)
        } else {
            self.write_unsigned_byte(0)
        }
    }
    pub(crate) fn write_float(&mut self, v: f32) -> IResult<()> {
        self.built_data.write_f32::<BigEndian>(v)
    }
    pub(crate) fn write_double(&mut self, v: f64) -> IResult<()> {
        self.built_data.write_f64::<BigEndian>(v)
    }
    pub(crate) fn write_bit_set(&mut self, bits: &[i64]) -> IResult<()> {
        self.write_var_int(bits.len() as i32)?;
        for b in bits {
            self.write_long(*b)?;
        }
        Ok(())
    }

    pub fn write_login_success(
        &mut self,
        uuid: uuid::Uuid,
        name: String,
        _other_players: (),
    ) -> IResult<()> {
        self.set_packet_id(PacketId::LoginSuccess);
        self.write_uuid(uuid)?;
        self.write_string(name)?;

        dbg!("TODO: other players");
        self.write_var_int(0)
    }

    pub fn write_login_play(&mut self, entity_id: EntityId, codec: nbt::Blob) -> IResult<()> {
        self.set_packet_id(PacketId::LoginPlay);

        self.write_int(entity_id.id)?; // TODO: entity ID.
        self.write_boolean(false)?;
        self.write_unsigned_byte(0)?;
        self.write_byte(-1)?;

        self.write_var_int(3)?;
        self.write_string("minecraft:overworld".to_string())?;
        self.write_string("minecraft:the_end".to_string())?;
        self.write_string("minecraft:nether".to_string())?;

        codec.to_writer(&mut self.built_data)?;

        self.write_string("minecraft:overworld".to_string())?;
        self.write_string("minecraft:overworld".to_string())?;

        self.write_long(0x100000)?;
        self.write_var_int(20)?;
        self.write_var_int(16)?;
        self.write_var_int(16)?;
        self.write_boolean(false)?;
        self.write_boolean(true)?;
        self.write_boolean(false)?;
        self.write_boolean(false)?;
        self.write_boolean(false)?;
        self.write_var_int(0)
    }

    pub fn write_spawn_player(&mut self, uuid: uuid::Uuid) -> IResult<()> {
        self.set_packet_id(PacketId::SpawnPlayer);

        self.write_var_int(10)?;
        self.write_uuid(uuid)?;
        self.write_double(0.)?;
        self.write_double(0.)?;
        self.write_double(0.)?;
        self.write_byte(0)?;
        self.write_byte(0)
    }

    pub fn write_change_difficulty(&mut self) -> IResult<()> {
        self.set_packet_id(PacketId::ChangeDifficulty);
        self.write_unsigned_byte(3)?;
        self.write_boolean(false)
    }

    pub fn write_spawn_entity(&mut self, uuid: uuid::Uuid) -> IResult<()> {
        self.set_packet_id(PacketId::SpawnEntify);
        self.write_var_int(10)?;
        self.write_uuid(uuid)?;
        self.write_var_int(0)?;
        self.write_double(0.)?;
        self.write_double(0.)?;
        self.write_double(0.)?;

        self.write_byte(0)?;
        self.write_byte(0)?;
        self.write_byte(0)?;

        self.write_var_int(0)?;

        self.write_short(0)?;
        self.write_short(0)?;
        self.write_short(0)?;

        Ok(())
    }

    pub fn write_feature_flags(&mut self, features: &[FeatureFlags]) -> IResult<()> {
        self.set_packet_id(PacketId::FeatureFlags);

        let length = features.len();
        self.write_var_int(length as i32)?;
        
        for s in features {
            self.write_string(s.to_ident().to_string())?;
        }

        Ok(())
    }

    pub fn write_plugin_message(&mut self, channel: String, data: &[u8]) -> IResult<()> {
        self.set_packet_id(PacketId::PluginMessage);

        self.write_string(channel)?;
        self.write_var_int(data.len() as i32)?;
        self.built_data.write_all(&data)
    }

    pub fn write_player_abilities(&mut self, flags: PlayerAbilitiesFlags, flying_speed: f32, field_of_view_modifier: f32) -> IResult<()> {
        self.set_packet_id(PacketId::PlayerAbilities);

        self.write_unsigned_byte(flags.bits())?;
        self.write_float(flying_speed)?;
        self.write_float(field_of_view_modifier)
    }

    pub fn write_held_item(&mut self, pos: u8) -> IResult<()> {
        if 8 < pos {
            println!("Position({}) is not support in vanilla", pos);
        }

        self.set_packet_id(PacketId::SetHeldItem);
        self.write_unsigned_byte(pos)
    }

    pub fn write_update_recipes(&mut self, recipes: &[Recipes]) -> IResult<()> {
        self.set_packet_id(PacketId::UpdateRecipes);

        self.write_var_int(recipes.len() as i32)?;

        println!("Recipe is not implemented yet");

        Ok(())
    }
    
    pub fn write_chunk_data_update_light(&mut self) -> IResult<()> {
        self.set_packet_id(PacketId::ChunkDataAndUpdateLight);

        self.write_int(0)?;
        self.write_int(0)?;

        let mut heightmaps = nbt::Blob::new();
        let array = nbt::Value::List(vec![nbt::Value::Long(0); 37]);
        heightmaps.insert("MOTION_BLOCKING", array.clone())?;
        // heightmaps.insert("WORLD_SURFACE", array)?;

        println!("{}", heightmaps);
        heightmaps.to_writer(&mut self.built_data)?;

        self.write_var_int(0)?;
        // Skip data
        
        self.write_var_int(0)?;
        // Skip block entity
        
        self.write_bit_set(&[0])?;
        self.write_bit_set(&[0])?;
        self.write_bit_set(&[0])?;
        self.write_bit_set(&[0])?;

        self.write_var_int(0)?;
        // Skip sky light array

        self.write_var_int(0)?;
        // Skip block light array

        Ok(())
    }

    pub fn write_respawn(&mut self) -> IResult<()> {
        self.set_packet_id(PacketId::Respawn);

        self.write_string("minecraft:overworld".to_string())?;
        self.write_string("minecraft:overworld".to_string())?;
        self.write_long(100)?;
        self.write_unsigned_byte(0)?;
        self.write_byte(-1)?;
        self.write_boolean(true)?;
        self.write_boolean(true)?;
        self.write_byte(0b01 | 0b10)?;
        self.write_boolean(false)?;
        self.write_var_int(10)?;

        Ok(())
    }

    pub fn write_synchronize_player_position(&mut self) -> IResult<()> {
        self.set_packet_id(PacketId::SynchronizePlayerPosition);

        self.write_double(0.)?;
        self.write_double(0.)?;
        self.write_double(0.)?;

        self.write_float(0.)?;
        self.write_float(0.)?;

        self.write_byte(7)?;
        self.write_var_int(0x55)?;

        Ok(())
    }
}
