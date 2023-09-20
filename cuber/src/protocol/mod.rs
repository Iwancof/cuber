pub mod client_bound;
pub mod primitive;
pub mod server_bound;

use std::io::{Cursor, Read, Write};
use tokio::io::{AsyncReadExt, AsyncWrite, BufReader, BufWriter};

use crate::protocol::primitive::leb128::async_read_var_int;

use client_bound::ClientBoundPacket;
use server_bound::{Handshaking, Login, PacketCluster, Play, Status};

pub type CResult<T> = Result<T, anyhow::Error>;

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

    pub async fn receive_packet(&mut self) -> CResult<ReceivedPacket> {
        assert_eq!(self.compression, Compression::Disabled); // TODO
        assert_eq!(self.encryption, Encryption::Disabled); // TODO

        receive_packet_plain_no_compression(&mut self.reader).await
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
    writer.write_all(&packet.buf).await.unwrap();

    packet.buf.len()
}

pub trait Encodable {
    fn encode<T: Write>(&self, writer: &mut T) -> usize;
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
    pub fn as_handshaking(mut self) -> CResult<Handshaking> {
        Handshaking::parse(&mut self)
    }
    pub fn as_status(mut self) -> CResult<Status> {
        Status::parse(&mut self)
    }
    pub fn as_login(mut self) -> CResult<Login> {
        Login::parse(&mut self)
    }
    pub fn as_play(mut self) -> CResult<Play> {
        Play::parse(&mut self)
    }
}

// TODO: change by connection configure.
pub async fn receive_packet_plain_no_compression<T: tokio::io::AsyncRead + Unpin>(
    reader: &mut T,
) -> CResult<ReceivedPacket> {
    let length = async_read_var_int(reader).await?.1 as _;
    let mut buffer = vec![0; length];

    reader.read_exact(&mut buffer).await?;

    Ok(ReceivedPacket {
        buf: Cursor::new(buffer.into_boxed_slice()),
    })
}

pub trait Decodable: Sized {
    fn decode<T: Read>(reader: &mut T) -> CResult<Self>;
}
