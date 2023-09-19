pub mod protocol;

use std::net::Ipv4Addr;

use protocol::receive_packet_plain_no_compress;
use protocol::CResult;
use tokio::{net::TcpListener};

#[tokio::main]
async fn main() -> CResult<()> {
    let listener = TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 25565)).await?;
    
    while let Ok((mut socket ,addr)) = listener.accept().await {
        println!("Connection from {addr}");
        let mut packet = receive_packet_plain_no_compress(&mut socket).await?;
        
        use protocol::server_bound::PacketCluster;
        let result = protocol::server_bound::Handshaking::parse(&mut packet)?;
        dbg!(result);
    }
    
    Ok(())
}
