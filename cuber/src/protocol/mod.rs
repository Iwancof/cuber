pub mod client_bound;
pub mod primitive;
pub mod server_bound;

use std::io::{BufWriter, Cursor, Read, Write};

use tokio::io::AsyncReadExt;

use crate::protocol::primitive::{VarInt, leb128::async_read_var_int};

pub type CResult<T> = Result<T, anyhow::Error>;

pub struct Client;

#[derive(Clone, Debug)]
pub struct BuiltPacket {
    buf: Box<[u8]>,
}

impl BuiltPacket {
    pub async fn send(self, _client: &mut Client) -> CResult<usize> {
        todo!()
    }
}

pub trait Encodable {
    fn encode<T: Write>(&self, writer: &mut T) -> usize;

    fn to_bytes(&self) -> Vec<u8> {
        let mut buf: BufWriter<Vec<u8>> = BufWriter::new(Vec::new());
        self.encode(&mut buf);

        buf.into_inner().unwrap()
    }
    fn to_packet(&self) -> BuiltPacket {
        BuiltPacket {
            buf: self.to_bytes().into_boxed_slice(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReceivedPacket {
    buf: Cursor<Box<[u8]>>,
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
pub async fn receive_packet_plain_no_compress<T: tokio::io::AsyncRead + Unpin>(reader: &mut T) -> CResult<ReceivedPacket> {
    let length = async_read_var_int(reader).await?.1 as _;
    let mut buffer = vec![0; length];
    
    reader.read_exact(&mut buffer).await?;
    
    Ok(ReceivedPacket {
        buf: Cursor::new(buffer.into_boxed_slice())
    })
}

pub trait Decodable: Sized {
    fn decode<T: Read>(reader: &mut T) -> CResult<Self>;
}
