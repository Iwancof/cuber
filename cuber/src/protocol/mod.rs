pub mod client_bound;
pub mod common;
pub mod primitive;
pub mod server_bound;

use std::io::{Cursor, Read, Write};
use tokio::io::{AsyncReadExt, AsyncWrite, BufReader, BufWriter};

use client_bound::ClientBoundPacket;
use common::*;
use primitive::leb128::{async_read_var_int, build_var_int};
use server_bound::{Handshaking, Login, PacketCluster, Play, Status};

pub use anyhow::Result;

pub trait Encodable {
    fn encode<T: Write>(&self, writer: &mut T) -> usize;
}
pub trait Decodable: Sized {
    fn decode<T: Read>(reader: &mut T) -> Result<Self>;
}

#[derive(Debug)]
pub struct Client {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>,

    state: State,
    compression: Compression,
    encryption: Encryption,
}

impl Client {
    pub fn from_stream(stream: tokio::net::TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        let reader = BufReader::new(reader);
        let writer = BufWriter::new(writer);

        Self {
            reader,
            writer,
            state: State::Handshaking,

            compression: Compression::Disabled,
            encryption: Encryption::Disabled,
        }
    }

    pub async fn send_built_packet(&mut self, packet: BuiltPacket) -> usize {
        assert_eq!(self.compression, Compression::Disabled); // TODO
        assert_eq!(self.encryption, Encryption::Disabled); // TODO

        send_packet_plain_no_compression(&mut self.writer, packet).await
    }
    pub async fn send_packet<T>(&mut self, packet: T) -> usize
    where
        T: ClientBoundPacket,
    {
        packet.verify(self.state);

        self.send_built_packet(packet.to_packet()).await
    }

    pub async fn receive_packet(&mut self) -> Result<ReceivedPacket> {
        assert_eq!(self.compression, Compression::Disabled); // TODO
        assert_eq!(self.encryption, Encryption::Disabled); // TODO

        receive_packet_plain_no_compression(&mut self.reader).await
    }
    pub fn set_state(&mut self, state: State) {
        self.state = state;
    }
}

#[derive(Clone, Debug)]
pub struct BuiltPacket {
    buf: Box<[u8]>, // plain data.
}

pub async fn send_packet_plain_no_compression<T: AsyncWrite + Unpin>(
    writer: &mut T,
    packet: BuiltPacket,
) -> usize {
    use tokio::io::AsyncWriteExt;
    let mut data = build_var_int(packet.buf.len() as _);
    data.extend_from_slice(&packet.buf);

    writer.write_all(&data).await.unwrap();

    packet.buf.len()
}

#[derive(Clone, Debug)]
pub struct ReceivedPacket {
    buf: Cursor<Box<[u8]>>, // plain data.
}

impl Read for ReceivedPacket {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.buf, buf)
    }
}

impl Drop for ReceivedPacket {
    fn drop(&mut self) {
        let remain = self.buf.get_ref().len() - self.buf.position() as usize;
        if remain != 0 {
            panic!("Unprocessed byte sequence remains. {} byte(s)", remain);
        }
    }
}

impl ReceivedPacket {
    pub fn as_handshaking(mut self) -> Result<Handshaking, (anyhow::Error, Self)> {
        Handshaking::parse(&mut self).map_err(|e| (e, self))
    }
    pub fn as_status(mut self) -> Result<Status, (anyhow::Error, Self)> {
        Status::parse(&mut self).map_err(|e| (e, self))
    }
    pub fn as_login(mut self) -> Result<Login, (anyhow::Error, Self)> {
        Login::parse(&mut self).map_err(|e| (e, self))
    }
    pub fn as_play(mut self) -> Result<Play, (anyhow::Error, Self)> {
        Play::parse(&mut self).map_err(|e| (e, self))
    }
}

// TODO: change by connection configure.
pub async fn receive_packet_plain_no_compression<T: tokio::io::AsyncRead + Unpin>(
    reader: &mut T,
) -> Result<ReceivedPacket> {
    let length = async_read_var_int(reader).await?.1 as _;
    let mut buffer = vec![0; length];

    reader.read_exact(&mut buffer).await?;

    Ok(ReceivedPacket {
        buf: Cursor::new(buffer.into_boxed_slice()),
    })
}
