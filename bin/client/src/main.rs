use std::{path::PathBuf, net::SocketAddr};

use agent_lib::{tls, AgentServiceClient, InstallPackageRequest, StartServiceRequest};
use structopt::StructOpt;
use tarpc::{client, context, tokio_serde::formats::Bincode};

#[derive(Debug, structopt::StructOpt)]
struct Args {
    #[structopt(default_value = "0.0.0.0:8081")]
    bind_addr: SocketAddr,
    #[structopt(default_value = "assets/localhost-cert.pem")]
    cert: PathBuf,
    #[structopt(subcommand)]
    rpc: Rpc,
}

#[derive(Debug, structopt::StructOpt)]
enum Rpc {
    Start(StartServiceRequest),
    InstallPackage(InstallPackageRequest),
}

#[derive(Debug, StructOpt)]
pub struct PutFile {
    target_path: PathBuf,
    source_file: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts: Args = structopt::StructOpt::from_args();
    // println!("opts {:?}", opts);
    let tls = tls::connect(&opts.bind_addr, &opts.cert)
        .await
        .unwrap();
    let transport = tarpc::serde_transport::Transport::from((tls, Bincode::default()));
    let client = AgentServiceClient::new(client::Config::default(), transport).spawn();

    match opts.rpc {
        Rpc::Start(start) => {
            let response = client.start_service(context::current(), start).await?;
            println!("called start and got response {:?}", response);
        }
        Rpc::InstallPackage(install) => {
            let response = client.install_package(context::current(), install).await?;
            println!("called install package and got response {:?}", response);
        }
    }
    Ok(())
}
