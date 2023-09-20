pub mod protocol;

use std::net::Ipv4Addr;

use nbt::Blob;
use protocol::client_bound::{LoginPlay, LoginSuccess};
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
            .as_handshaking()
            .unwrap()
            .unwrap_handshake();
        if result.next_state != HandshakeNextState::Login {
            continue;
        }
        client.set_state(protocol::common::State::Login);

        let ls = client
            .receive_packet()
            .await?
            .as_login()
            .unwrap()
            .assume_login_start()?;

        println!("New player!  name: {}, uuid: {:?}", ls.name, ls.uuid.0);

        let sc = LoginSuccess {
            uuid: ls.uuid.0.unwrap_or_default(),
            user_name: ls.name,
            property: vec![].into(),
        };
        client.send_packet(sc).await;

        client.set_state(protocol::common::State::Play);

        let login_play = LoginPlay {
            entity_id: 0x11223344,
            is_hardcore: false,
            game_mode: protocol::common::GameMode::Creative,
            previous_game_mode: protocol::common::GameMode::Undefined,
            dimension_names: vec![
                "minecraft:overworld".into(),
                "minecraft:the_end".into(),
                "minecraft:nether".into(),
            ]
            .into(),
            registry_codec: read_mock_nbt_blob()?,
            // registry_codec: Blob::new(),
            dimension_type: "minecraft:overworld".into(),
            dimension_name: "minecraft:overworld".into(),
            hashed_seed: 0x100000,
            max_players: 20.into(),
            view_distance: 10.into(),
            simulation_distance: 10.into(),
            reduce_debug_info: false,
            enable_respawn_screen: true,
            is_debug: false,
            is_flat: false,
            death_location: None.into(),
            portal_cooldown: 10.into(),
        };
        client.send_packet(login_play).await;

        while true {
            let packet = client.receive_packet().await?.as_play();
            println!("packet: {:?}", packet);
        }
    }

    Ok(())
}
fn read_mock_nbt_blob() -> std::io::Result<nbt::Blob> {
    use std::fs;

    let mut f = fs::File::open("../1_20_1_codec.bin")?;
    let r = Ok(nbt::de::from_reader(&mut f)?);

    // let mut remain = vec![];
    // f.read_to_end(&mut remain).unwrap();

    // let mut f = fs::File::create("result")?;
    // f.write_all(&remain).unwrap();

    r
}
