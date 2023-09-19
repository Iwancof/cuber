pub mod client_bound;
pub mod primitive;
pub mod server_bound;

use std::io::{BufWriter, Cursor, Read, Write};

use tokio::io::{AsyncReadExt, AsyncWrite};

use crate::protocol::primitive::{leb128::async_read_var_int, VarInt};

pub type CResult<T> = Result<T, anyhow::Error>;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub enum State {
    Handshaking,
    Status,
    Login,
    Play,
}

pub struct Client;

#[derive(Clone, Debug)]
pub struct BuiltPacket {
    buf: Box<[u8]>, // plain data.
}

// TODO: replace `T` with `Client`
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
