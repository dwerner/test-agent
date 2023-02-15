use std::net::{IpAddr, Ipv6Addr};

use agent_lib::{tls, AgentServiceClient, InstallPackageRequest, StartNodeRequest};
use structopt::StructOpt;
use tarpc::{client, context, tokio_serde::formats::Bincode};

#[derive(Debug, structopt::StructOpt)]
enum Args {
    Start(StartNodeRequest),
    InstallPackage(InstallPackageRequest),
}

#[derive(Debug, StructOpt)]
enum AgentRequest {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // let opts: Args = structopt::StructOpt::from_args();
    // println!("opts {:?}", opts);
    let tls = tls::connect("127.0.0.1", 8081, "assets/cert.pem")
        .await
        .unwrap();
    let transport = tarpc::serde_transport::Transport::from((tls, Bincode::default()));
    let client = AgentServiceClient::new(client::Config::default(), transport).spawn();

    let response = client
        .start_node(context::current(), StartNodeRequest { wrapper: None })
        .await?;

    println!("response {:?}", response);

    // match opts {
    //     Args::Start(start) => {
    //         let response = client.start_node(context::current(), start).await?;
    //         println!("called message and got response {:?}", response);
    //     }
    //     Args::InstallPackage(install) => {
    //         let response = client.install_package(context::current(), install).await?;
    //         println!("called message and got response {:?}", response);
    //     }
    // }
    Ok(())
}
