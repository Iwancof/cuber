#![feature(cursor_remaining)]

use std::io::{Read, Result as IResult};
use std::net::Ipv4Addr;

use byteorder::{ReadBytesExt, BigEndian};
use tokio::io::Result as TIResult;
use tokio::net::{TcpListener, TcpStream};

#[macro_use]
extern crate num_derive;

#[derive(Debug)]
struct MinecraftClient {
    socket: TcpStream,
    state: ClientState,
}

#[derive(Debug)]
enum ClientState {
    Initial,
    Handshaking,
}

#[derive(Debug)]
struct PacketParser {
    packet_data: std::io::Cursor<Vec<u8>>,
}

const SEGMENT_BITS: u8 = 0x7f;
const CONTINUE_BIT: u8 = 0x80;
async fn async_read_var_int<T: tokio::io::AsyncReadExt + std::marker::Unpin>(
    d: &mut T,
) -> TIResult<(usize, i32)> {
    let mut value = 0;
    let mut position = 0;
    let mut read = 0;

    loop {
        let current_byte = d.read_u8().await?;
        read += 1;

        let segment = current_byte & SEGMENT_BITS;
        value |= (segment as i32) << position;

        if current_byte & CONTINUE_BIT == 0 {
            break;
        }

        position += 7;
        if position >= 32 {
            return Err(tokio::io::Error::new(
                tokio::io::ErrorKind::InvalidData,
                "VarInt is too big",
            ));
        }
    }

    Ok((read, value))
}

fn read_var_int<T: ReadBytesExt>(d: &mut T) -> IResult<(usize, i32)> {
    let mut value = 0;
    let mut position = 0;
    let mut read = 0;

    loop {
        let current_byte = d.read_u8()?;
        read += 1;

        let segment = current_byte & SEGMENT_BITS;
        value |= (segment as i32) << position;

        if current_byte & CONTINUE_BIT == 0 {
            break;
        }

        position += 7;
        if position >= 32 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "VarInt is too big",
            ));
        }
    }

    Ok((read, value))
}

#[derive(FromPrimitive, Debug, Copy, Clone, PartialEq, Eq)]
enum PacketId {
    Handshaking = 0x00,
}

impl MinecraftClient {
    fn new(socket: TcpStream) -> Self {
        Self { socket, state: ClientState::Initial }
    }

    async fn read_packet_size(&mut self) -> TIResult<usize> {
        let (_, packet_size) = async_read_var_int(&mut self.socket).await?;

        Ok(packet_size as usize)
    }

    async fn get_packet(&mut self) -> TIResult<PacketParser> {
        use tokio::io::AsyncReadExt;

        let packet_size = self.read_packet_size().await?;

        let mut buf = vec![0u8; packet_size];
        self.socket.read_exact(&mut buf).await?;

        Ok(PacketParser {
            packet_data: std::io::Cursor::new(buf),
        })
    }
}

impl PacketParser {
    fn read_unsigned_short(&mut self) -> IResult<u16> {
        self.packet_data.read_u16::<byteorder::BigEndian>()
    }
    fn read_var_int(&mut self) -> IResult<i32> {
        let (_read, data) = read_var_int(&mut self.packet_data)?;
        Ok(data)
    }
    fn read_string(&mut self) -> IResult<String> {
        let size = self.read_var_int()? as usize;
        let mut string_buf = vec![0u8; size];
        self.packet_data.read_exact(&mut string_buf)?;

        Ok(String::from_utf8(string_buf)
            .map_err(|_| (std::io::Error::new(std::io::ErrorKind::InvalidData, "Unicode error")))?)
    }
    fn read_uuid(&mut self) -> IResult<uuid::Uuid> {
        let raw = self.packet_data.read_u128::<BigEndian>()?;
        Ok(uuid::Uuid::from_u128(raw))
    }

    fn read_packet_id(&mut self) -> IResult<PacketId> {
        use num_traits::FromPrimitive;

        match FromPrimitive::from_i32(self.read_var_int()?) {
            None => Err(tokio::io::Error::new(
                tokio::io::ErrorKind::InvalidData,
                "Unknown packet ID",
            )),
            Some(val) => Ok(val),
        }
    }
    fn read_version(&mut self) -> IResult<i32> {
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
    fn get_next_state(&mut self) -> IResult<HandshakeNextState> {
        use num_traits::FromPrimitive;

        match FromPrimitive::from_i32(self.read_var_int()?) {
            None => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unknown handshake next state",
            )),
            Some(val) => Ok(val),
        }
    }
}

#[derive(FromPrimitive, Debug)]
pub enum HandshakeNextState {
    Status = 1,
    Login = 2,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 25565))
        .await
        .unwrap();

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New client. from {:?}", addr);

        let mut client = MinecraftClient::new(socket);

        let mut packet = client.get_packet().await?;
        packet.read_packet_id()?;
        packet.read_version()?;
        // println!("[Handshaking] The version is 1.20.1");

        let host = packet.read_string()?;
        let port = packet.read_unsigned_short()?;
        println!("Attempt to connect to {}:{}", host, port);

        let state = packet.get_next_state()?;
        println!("next: {:?}", state);
        
        let mut packet = client.get_packet().await?;
        println!("[{:?}] The player name is {}({})", packet.read_packet_id()?, packet.read_string()?, packet.read_uuid()?);
    }
}
