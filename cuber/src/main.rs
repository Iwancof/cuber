pub mod protocol;

use std::marker::PhantomData;
use std::net::Ipv4Addr;

use protocol::client_bound::ClientBoundPacket;
use protocol::client_bound::LoginSuccess;
use protocol::primitive::VarInt;
use protocol::receive_packet_plain_no_compression;
use protocol::send_packet_plain_no_compression;
use protocol::server_bound::HandshakeNextState;
use protocol::server_bound::Handshaking;
use protocol::server_bound::{Login, LoginStart};
use protocol::CResult;
use tokio::net::TcpListener;

use protocol::primitive::Array;

#[tokio::main]
async fn main() -> CResult<()> {
    let listener = TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 25565)).await?;

    while let Ok((mut socket, addr)) = listener.accept().await {
        println!("Connection from {addr}");
        use protocol::server_bound::PacketCluster;

        let mut packet = receive_packet_plain_no_compression(&mut socket).await?;
        let result = Handshaking::parse(&mut packet)?;
        if let Handshaking::Handshake(hs) = result {
            if hs.next_state != HandshakeNextState::Login {
                continue;
            }
        } else {
            panic!("???");
        }

        let mut packet = receive_packet_plain_no_compression(&mut socket).await?;
        let ls = Login::parse(&mut packet)?;

        dbg!(&&ls);

        if let Login::LoginStart(ls) = ls {
            let sc = LoginSuccess {
                uuid: ls.uuid.0.unwrap_or_default(),
                user_name: ls.name,
                property: vec![].into(),
            };
            dbg!(&&sc);
            sc.verify(protocol::State::Login);

            let p = sc.to_packet();
            send_packet_plain_no_compression(&mut socket, p).await;
        }
    }

    Ok(())
}
