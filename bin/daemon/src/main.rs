use std::{io::BufRead, net::SocketAddr};

use agent_lib::{
    pkg_manager, tls, AgentService, InstallPackageRequest, InstallPackageResponse,
    StartNodeRequest, StartNodeResponse,
};
use futures::{future, StreamExt};
use tarpc::{
    server::{self, Channel},
    tokio_serde::formats::Bincode,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // sudo::escalate_if_needed().unwrap();

    // println!("Successfully escalated privileges...");

    let listener = tls::serve(
        "0.0.0.0 TODO ME",
        8081,
        "assets/localhost-cert.pem",
        "assets/localhost-key.pem",
        Bincode::default,
    )
    .await?;

    listener
        .filter_map(|r| {
            let transport = match r {
                Ok(transport) => transport,
                Err(err) => {
                    println!("error with transport : {:?}", err);
                    return future::ready(None);
                }
            };
            println!("got a transport");
            future::ready(Some(transport))
        })
        .map(server::BaseChannel::with_defaults)
        //.max_channels_per_key(1, |t| t.transport().peer_addr().unwrap().ip())
        .map(|channel| {
            println!("creating a channel");
            let server = Agent::new(channel.transport().peer_addr().unwrap())
                .expect("unable to create agent");
            channel.execute(server.serve())
        })
        .buffer_unordered(10)
        .for_each(|_| async {
            println!("did something ....");
        })
        .await;

    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum AgentError {}

#[derive(Clone)]
struct Agent {
    _addr: SocketAddr,
}

impl Agent {
    fn new(addr: SocketAddr) -> Result<Self, AgentError> {
        Ok(Self { _addr: addr })
    }
}

#[tarpc::server]
impl AgentService for Agent {
    async fn start_node(
        self,
        _: tarpc::context::Context,
        _request: StartNodeRequest,
    ) -> StartNodeResponse {
        for i in 0..10 {
            println!("did some work {i}");
        }
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
