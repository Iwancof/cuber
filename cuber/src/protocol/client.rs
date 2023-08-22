use super::data_types::async_read_var_int;
use super::packet::PacketBuilder;
use super::TIResult;

#[derive(Debug)]
pub struct MinecraftClient {
    socket: tokio::net::TcpStream,
    state: ClientState,
}

#[derive(Debug, Clone, Copy)]
pub enum ClientState {
    Initial,
    Handshaking,
    Status,
    Play,
    Login,
}

impl MinecraftClient {
    pub fn new(socket: tokio::net::TcpStream) -> Self {
        Self {
            socket,
            state: ClientState::Initial,
        }
    }

    pub(crate) fn set_state(&mut self, new: ClientState) {
        self.state = new;
    }

    pub(crate) async fn read_packet_size(&mut self) -> TIResult<usize> {
        let (_, packet_size) = async_read_var_int(&mut self.socket).await?;

        Ok(packet_size as usize)
    }

    pub(crate) async fn get_packet(&mut self) -> TIResult<super::packet::PacketParser> {
        use super::packet::PacketParser;
        use tokio::io::AsyncReadExt;

        let packet_size = self.read_packet_size().await?;

        let mut buf = vec![0u8; packet_size];
        self.socket.read_exact(&mut buf).await?;

        Ok(PacketParser::from_state_vec(self.state, buf))
    }
    pub(crate) async fn send_packet(&mut self, pb: PacketBuilder) -> TIResult<usize> {
        use tokio::io::AsyncWriteExt;

        let data = pb.to_packet_bytes();
        let l = data.len();
        dbg!("[sent]");
        println!("{:?}", data);
        self.socket.write_all(&data).await?;

        Ok(l)
    }
}
