pub mod client_bound;
pub mod primitive;
pub mod server_bound;

use std::io::{BufWriter, Cursor, Read, Write};

type CResult<T> = Result<T, anyhow::Error>;

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

pub trait Decodable: Sized {
    fn decode<T: Read>(reader: &mut T) -> CResult<Self>;

    fn from_packet(mut packet: ReceivedPacket) -> CResult<Self> {
        let object = Self::decode(&mut packet.buf)?;
        let remain = packet.buf.get_ref().len() - packet.buf.position() as usize;

        assert_eq!(remain , 0);

        Ok(object)
    }
}
