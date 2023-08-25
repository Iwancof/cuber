#![feature(cursor_remaining)]

pub mod protocol;

use protocol::packet::PacketBuilder;

use std::{io, net::Ipv4Addr};
use tokio::net::TcpListener;

use crate::protocol::packet::EntityId;

#[macro_use]
extern crate num_derive;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    read_test();

    let listener = TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 25565))
        .await
        .unwrap();

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New client. from {:?}", addr);

        let mut client = protocol::client::MinecraftClient::new(socket);
        client.set_state(protocol::client::ClientState::Handshaking);

        let mut packet = client.get_packet().await?;
        packet.handle_handshake()?;
        client.set_state(protocol::client::ClientState::Login);

        let mut packet = client.get_packet().await?;
        let (name, uuid) = packet.handle_login_start()?;
        let uuid = uuid.unwrap(); // TODO: generate UUID
        println!("The player name is {}({})", name, uuid,);

        let mut pb = PacketBuilder::new();
        pb.write_login_success(uuid, name, ())?;
        client.send_packet(pb).await?;

        client.set_state(protocol::client::ClientState::Play);
        let mut pb = PacketBuilder::new();
        pb.write_login_play(EntityId { id: 10 }, read_mock_nbt_blob()?)?;
        client.send_packet(pb).await?;

        let mut pb = PacketBuilder::new();
        pb.write_feature_flags(&[protocol::packet::FeatureFlags::Vanilla])?;
        client.send_packet(pb).await?;

        let mut pb = PacketBuilder::new();
        pb.write_plugin_message("minecraft:brand".to_string(), "vanilla".as_bytes())?;
        client.send_packet(pb).await?;

        let mut pb = PacketBuilder::new();
        pb.write_change_difficulty()?;
        client.send_packet(pb).await?;

        let mut pb = PacketBuilder::new();
        pb.write_player_abilities(protocol::packet::PlayerAbilitiesFlags::empty(), 0.05, 0.1)?;
        client.send_packet(pb).await?;

        let mut pb = PacketBuilder::new();
        pb.write_held_item(0)?;
        client.send_packet(pb).await?;

        /*
        let mut pb = PacketBuilder::new();
        pb.write_chunk_data_update_light()?;
        client.send_packet(pb).await?;
        */

        /*
        let mut pb = PacketBuilder::new();
        pb.write_synchronize_player_position()?;
        client.send_packet(pb).await?;
        */

        /*
        let mut pb = PacketBuilder::new();
        pb.write_spawn_player(uuid)?;
        client.send_packet(pb).await?;
        */
        let mut pb = PacketBuilder::new();
        pb.write_spawn_entity(uuid)?;
        client.send_packet(pb).await?;

        /*
        let mut pb = PacketBuilder::new();
        pb.write_respawn()?;
        client.send_packet(pb).await?;
        */

        println!("sent all packets");

        loop {
            let packet = client.get_packet().await?;
            println!("{:?}", packet);

            let mut pb = PacketBuilder::new();
            pb.write_synchronize_player_position()?;
            client.send_packet(pb).await?;
        }
    }
}

fn read_mock_nbt_blob() -> std::io::Result<nbt::Blob> {
    use std::fs;
    use std::io::{Read, Write};

    let mut f = fs::File::open("../1_20_1_codec.bin")?;
    let r = Ok(nbt::de::from_reader(&mut f)?);

    let mut remain = vec![];
    f.read_to_end(&mut remain).unwrap();

    let mut f = fs::File::create("result")?;
    f.write_all(&remain).unwrap();

    r
}

fn read_test() {
    use std::fs;
    use std::io::{Read, Write};
    let mut f = fs::File::open("../chunk_data.bin").unwrap();
    let r: nbt::Blob = nbt::de::from_reader(&mut f).unwrap();

    println!("{}", r);
    let mut remain = vec![];
    f.read_to_end(&mut remain).unwrap();

    let mut f = fs::File::create("result").unwrap();
    f.write_all(&remain).unwrap();
}
