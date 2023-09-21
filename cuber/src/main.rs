pub mod protocol;

use std::net::Ipv4Addr;

use protocol::client_bound::{
    ChangeDifficulty, FeatureFlags, LoginPlay, LoginSuccess, PlayerAbilities, PluginMessage,
    SetHeldItem, SpawnEntity,
};
use protocol::common::PlayerAbilitiesFlags;
use protocol::primitive::Angle;
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

        let player_uuid = ls.uuid.0.unwrap_or_default();

        println!("New player!  name: {}, uuid: {:?}", ls.name, ls.uuid.0);

        let sc = LoginSuccess {
            uuid: player_uuid.clone(),
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

        let features = FeatureFlags {
            features: vec![protocol::common::Feature::Vanilla].into(),
        };
        client.send_packet(features).await;

        let pm = PluginMessage {
            channel: "minecraft:brand".into(),
            data: "vanilla".as_bytes().into(),
        };
        client.send_packet(pm).await;

        let cd = ChangeDifficulty {
            new_difficulty: protocol::common::Difficulty::Peaceful,
        };
        client.send_packet(cd).await;

        let pa = PlayerAbilities {
            flags: PlayerAbilitiesFlags::CREATIVE_MODE,
            flying_speed: 0.1,
            field_of_view_modifier: 0.1,
        };
        client.send_packet(pa).await;

        let hi = SetHeldItem { slot: 0 };
        client.send_packet(hi).await;

        let se = SpawnEntity {
            entity_id: 0.into(),
            entity_uuid: player_uuid.clone(),
            mob_type: 0.into(),
            x: 0.,
            y: 0.,
            z: 0.,
            pitch: Angle { value: 0 },
            yaw: Angle { value: 0 },
            head_yaw: Angle { value: 0 },
            data: 0.into(),
            velocity_x: 0,
            velocity_y: 0,
            velocity_z: 0,
        };
        client.send_packet(se).await;

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
