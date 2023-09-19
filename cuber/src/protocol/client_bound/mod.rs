use std::io::BufWriter;

use deriver::Encodable;
use packet_id::cb_packet;
use structstruck;
use uuid::Uuid;

use super::{
    primitive::{Array, BoolConditional, Chat, Identifier, VarInt},
    BuiltPacket, Encodable, State,
};

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

#[cb_packet(State::Status, 0)]
#[derive(Encodable, Debug, PartialEq, Eq, Clone)]
pub struct StatusResponse {
    json_response: String, // replace with Json object.
}

#[cb_packet(State::Login, 0)]
#[derive(Encodable, Debug, PartialEq, Eq, Clone)]
pub struct Disconnect {
    chat: Chat,
}

#[cb_packet(State::Login, 1)]
#[derive(Encodable, Debug, PartialEq, Eq, Clone)]
pub struct EncryptionRequest {
    server_id: String,
    public_key: Array<VarInt, u8>,
    verify_token: Array<VarInt, u8>,
}

structstruck::strike! {
    #[cb_packet(State::Login, 0x02)]
    #[derive(Encodable, Debug, PartialEq, Eq, Clone)]
    pub struct LoginSuccess {
        uuid: Uuid,
        user_name: String,
        property: Array<VarInt, #[derive(Encodable, Debug, PartialEq, Eq, Clone)] pub struct LoginSuccessProperty {
            name: String,
            value: String,
            signature: BoolConditional<String>,
        }>,
    }
}

#[cb_packet(State::Login, 0x03)]
#[derive(Encodable, Debug, PartialEq, Eq, Clone)]
pub struct SetCompression {
    threshold: VarInt,
}

#[cb_packet(State::Login, 0x04)]
#[derive(Encodable, Debug, PartialEq, Eq, Clone)]
pub struct PluginRequest {
    message_id: VarInt,
    channel: Identifier,
    data: Array<VarInt, u8>,
}
