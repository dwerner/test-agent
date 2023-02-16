use std::{io::BufRead, net::SocketAddr, path::PathBuf};

use agent_lib::{
    pkg_manager, tls, AgentService, FetchFileRequest, FetchFileResponse, InstallPackageRequest,
    InstallPackageResponse, PutFileRequest, PutFileResponse, StartServiceRequest,
    StartServiceResponse,
};
use futures::{future, StreamExt};
use structopt::StructOpt;
use tarpc::{
    server::{self, Channel},
    tokio_serde::formats::Bincode,
};

#[derive(Debug, StructOpt)]
enum Args {
    Serve {
        #[structopt(default_value = "0.0.0.0:8081")]
        addr: SocketAddr,
        #[structopt(default_value = "assets/localhost-cert.pem")]
        cert: PathBuf,
        #[structopt(default_value = "assets/localhost-key.pem")]
        key: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::from_args();

    let Args::Serve { addr, cert, key } = args;
    //sudo::escalate_if_needed().unwrap();
    // println!("Successfully escalated privileges...");
    let listener = tls::serve(addr, cert, key, Bincode::default).await?;
    listener
        .filter_map(|r| {
            let transport = match r {
                Ok(transport) => transport,
                Err(err) => {
                    println!("error with transport : {:?}", err);
                    return future::ready(None);
                }
            };
            future::ready(Some(transport))
        })
        .map(server::BaseChannel::with_defaults)
        //.max_channels_per_key(1, |t| t.transport().peer_addr().unwrap().ip())
        .map(|channel| {
            println!("creating a new channel");
            let server = Agent::new(
                channel
                    .transport()
                    .peer_addr()
                    .expect("TODO: handle client closed connection"),
            )
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
    async fn put_file(self, _ctx: tarpc::context::Context, req: PutFileRequest) -> PutFileResponse {
        let PutFileRequest {
            target_path,
            target_perms,
            file,
        } = req;

        let (tempfile, temp_path) = file
            .into_temp_file_on_disk()
            .expect("TODO - unable to write temp file");
        drop(tempfile); // TODO: maybe we dont' pass it back
        println!(
            "would write to disk: {}, with perms {target_perms} temp file in {}",
            temp_path.display(),
            target_path.display()
        );
        PutFileResponse::Success
    }

    async fn fetch_file(
        self,
        _ctx: tarpc::context::Context,
        _req: FetchFileRequest,
    ) -> FetchFileResponse {
        todo!()
    }
    async fn stop_service(
        self,
        _ctx: tarpc::context::Context,
        _request: StartServiceRequest,
    ) -> StartServiceResponse {
        todo!()
    }

    async fn start_service(
        self,
        _: tarpc::context::Context,
        _request: StartServiceRequest,
    ) -> StartServiceResponse {
        for i in 0..10 {
            println!("did some work {i}");
        }
        StartServiceResponse::Error
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
