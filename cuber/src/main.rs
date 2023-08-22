#![feature(cursor_remaining)]

pub mod protocol;

use protocol::data_types::async_read_var_int;
use protocol::packet::PacketBuilder;

use std::net::Ipv4Addr;
use tokio::io::Result as TIResult;
use tokio::net::{TcpListener, TcpStream};

use crate::protocol::packet::packet_id::PacketId;

#[macro_use]
extern crate num_derive;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        pb.write_login_play()?;
        client.send_packet(pb.clone()).await?;

        client.get_packet().await?;
    }
}
