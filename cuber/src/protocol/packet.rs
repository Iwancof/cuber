pub mod packet_id;

use std::collections::HashMap;

use packet_id::PacketId;

use super::client::ClientState;
use super::data_types::*;
use super::IResult;
use byteorder::LittleEndian;
use byteorder::WriteBytesExt;
use byteorder::{BigEndian, ReadBytesExt};
use num_traits::ToPrimitive;

use std::io::Cursor;
use std::io::Read;

#[derive(ToPrimitive, FromPrimitive, Debug, PartialEq, Eq)]
pub enum HandshakeNextState {
    Status = 1,
    Login = 2,
}

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
        use num_traits::FromPrimitive;

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
        use std::io::Write;
        let data = build_var_int(val);
        self.built_data.write_all(&data)
    }
    pub(crate) fn write_string(&mut self, s: String) -> IResult<()> {
        use std::io::Write;
        let string_length = s.len();
        self.write_var_int(string_length as i32)?;
        self.built_data.write_all(&s.as_bytes()[..string_length])
    }
    pub(crate) fn write_uuid(&mut self, uuid: uuid::Uuid) -> IResult<()> {
        self.built_data
            .write_u128::<LittleEndian>(uuid.to_u128_le())
    }
    pub(crate) fn write_int(&mut self, val: i32) -> IResult<()> {
        self.built_data.write_i32::<BigEndian>(val)
    }
    pub(crate) fn write_unsigned_byte(&mut self, byte: u8) -> IResult<()> {
        self.built_data.write_u8(byte)
    }
    pub(crate) fn write_byte(&mut self, byte: i8) -> IResult<()> {
        self.built_data.write_i8(byte)
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

    pub fn write_login_play(&mut self) -> IResult<()> {
        self.set_packet_id(PacketId::LoginPlay);

        self.write_int(0)?; // TODO: entity ID.
        self.write_boolean(false)?;
        self.write_unsigned_byte(0)?;
        self.write_byte(-1)?;

        self.write_var_int(1)?;
        self.write_string("overworld".to_string())?;

        let codec = nbt::Blob::new();
        codec.to_writer(&mut self.built_data)?;

        self.write_string("natural".to_string())?;
        self.write_string("natural".to_string())?;

        self.write_long(0x1000)?;
        self.write_var_int(100)?;
        self.write_var_int(3)?;
        self.write_var_int(3)?;
        self.write_boolean(false)?;
        self.write_boolean(true)?;
        self.write_boolean(true)?;
        self.write_boolean(true)?;
        self.write_boolean(false)?;
        self.write_var_int(0)
    }
}
