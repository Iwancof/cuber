pub mod protocol;

use std::net::Ipv4Addr;

use protocol::client_bound::LoginSuccess;
use protocol::server_bound::HandshakeNextState;
use protocol::{CResult, Client};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> CResult<()> {
    let listener = TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 25565)).await?;

    while let Ok((socket, addr)) = listener.accept().await {
        println!("Connection from {addr}");

        let mut client = Client::from_stream(socket);

        let result = client
            .receive_packet()
            .await?
            .as_handshaking()?
            .unwrap_handshake();
        if result.next_state != HandshakeNextState::Login {
            continue;
        }

        let ls = client
            .receive_packet()
            .await?
            .as_login()?
            .assume_login_start()?;

        dbg!(&&ls);

        let sc = LoginSuccess {
            uuid: ls.uuid.0.unwrap_or_default(),
            user_name: ls.name,
            property: vec![].into(),
        };
        dbg!(&&sc);

        client.send_packet(sc).await;
    }

    Ok(())
}
