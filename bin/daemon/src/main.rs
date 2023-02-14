use std::net::{IpAddr, Ipv6Addr, SocketAddr};

use agent_lib::AgentService;
use futures::{future, StreamExt};
use tarpc::{
    server::{self, incoming::Incoming, Channel},
    tokio_serde::formats::Bincode,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server_addr = (IpAddr::V6(Ipv6Addr::LOCALHOST), 8081);
    let mut listener = tarpc::serde_transport::tcp::listen(&server_addr, Bincode::default).await?;
    listener
        .config_mut()
        .max_frame_length(std::u32::MAX as usize);

    listener
        .filter_map(|r| future::ready(r.ok()))
        .map(server::BaseChannel::with_defaults)
        .max_channels_per_key(1, |t| t.transport().peer_addr().unwrap().ip())
        .map(|channel| {
            let server = Agent(channel.transport().peer_addr().unwrap());
            channel.execute(server.serve())
        })
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;

    Ok(())
}

#[derive(Clone)]
struct Agent(SocketAddr);

#[tarpc::server]
impl AgentService for Agent {
    async fn message(
        self,
        _: tarpc::context::Context,
        _msg: agent_lib::Message,
    ) -> agent_lib::Response {
        let cmd = apt_cmd::AptGet::new();
        let install = cmd.force().install(&["linux-perf-tools"]).await;
        match install {
            Ok(_) => agent_lib::Response::Success,
            Err(err) => {
                println!("{:?}", err);
                agent_lib::Response::Error
            }
        }
    }
}
