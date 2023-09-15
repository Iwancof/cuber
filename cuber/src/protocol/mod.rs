pub mod client_bound;
pub mod server_bound;
pub mod primitive;

use std::io::{Write, BufWriter};

pub use std::io::Result as SResult;
pub use tokio::io::Result as AResult;

struct Client;

#[derive(Clone, Debug)]
struct Packet {
    buf: Box<[u8]>,
}

impl Packet {
    async fn send(self, _client: &mut Client) -> AResult<usize> {
        todo!()
    }
}

trait Encodable {
    fn encode<T: Write>(&self, writer: &mut T) -> usize;

    fn to_bytes(&self) -> Vec<u8> {
        let mut buf: BufWriter<Vec<u8>> = BufWriter::new(Vec::new());
        self.encode(buf);

        buf.into_inner().unwrap()
    }
    fn to_packet(&self) -> Packet {
        Packet { buf: self.to_bytes().into_boxed_slice() }
    }
}

