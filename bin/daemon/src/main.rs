use std::{
    io::BufRead,
    net::{IpAddr, Ipv6Addr, SocketAddr},
};

use agent_lib::{
    pkg_manager, AgentService, InstallPackageRequest, InstallPackageResponse, StartNodeRequest,
    StartNodeResponse,
};
use futures::{future, StreamExt};
use tarpc::{
    server::{self, incoming::Incoming, Channel},
    tokio_serde::formats::Bincode,
};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    sudo::escalate_if_needed().unwrap();
    println!("Successfully escalated privileges...");
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
            let server = Agent::new(channel.transport().peer_addr().unwrap())
                .expect("unable to create agent");
            channel.execute(server.serve())
        })
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;

    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum AgentError {}

#[derive(Clone)]
struct Agent {
    addr: SocketAddr,
}

impl Agent {
    fn new(addr: SocketAddr) -> Result<Self, AgentError> {
        Ok(Self { addr })
    }
}

#[tarpc::server]
impl AgentService for Agent {
    async fn start_node(
        self,
        _: tarpc::context::Context,
        request: StartNodeRequest,
    ) -> StartNodeResponse {
        
        StartNodeResponse::Error
    }

    async fn install_package(
        self,
        _: tarpc::context::Context,
        request: agent_lib::InstallPackageRequest,
    ) -> InstallPackageResponse {
        let InstallPackageRequest { name: pkg_name } = request;
        let mut mgr = pkg_manager::PkgWrapper::new(true).expect("unable to detect package manager");
        if !mgr.is_installed(&pkg_name) {
            println!("package {pkg_name} not found, installing");
            match mgr.install_pkg(&pkg_name) {
                Ok(reader) => {
                    for line in reader.lines() {
                        println!("child process output: {}", line.unwrap());
                    }
                    println!("successfully installed package {pkg_name}");
                    return InstallPackageResponse::Success;
                }
                _ => return InstallPackageResponse::Error,
            }
        } else {
            println!("package {pkg_name} already installed");
            return InstallPackageResponse::AlreadyInstalled;
        }
    }
}
