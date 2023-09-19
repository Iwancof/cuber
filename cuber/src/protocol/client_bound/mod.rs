use std::io::BufWriter;

use deriver::Encodable;
use packet_id::cb_packet;

use super::{BuiltPacket, Encodable, State};

pub trait ClientBoundPacket: Encodable {
    const PACKET_ID: i32;
    const VALID_STATE: State;

    fn verify(current_state: State) {
        assert_eq!(current_state, Self::VALID_STATE);
    }
    fn to_bytes(&self) -> Box<[u8]> {
        let mut buf: BufWriter<Vec<u8>> = BufWriter::new(Vec::new());
        self.encode(&mut buf);

        buf.into_inner().unwrap().into_boxed_slice()
    }
    fn to_packet(&self) -> BuiltPacket {
        BuiltPacket {
            buf: self.to_bytes(),
        }
    }
}

#[cb_packet(State::Handshaking, 0)]
#[derive(Encodable)]
struct Test {
    test: i32,
}
