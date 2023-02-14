use std::net::{IpAddr, Ipv6Addr};

use agent_lib::Message;
use tarpc::{client, context, tokio_serde::formats::Bincode};

#[derive(Debug, structopt::StructOpt)]
struct CliOpts {
    #[structopt(short = "h", long)]
    help: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts: CliOpts = structopt::StructOpt::from_args();
    println!("Opts {:?}", opts);
    let server_addr = (IpAddr::V6(Ipv6Addr::LOCALHOST), 8081);
    let transport = tarpc::serde_transport::tcp::connect(&server_addr, Bincode::default);
    let client =
        agent_lib::AgentServiceClient::new(client::Config::default(), transport.await?).spawn();
    let response = client
        .message(
            context::current(),
            Message::HelloWorld("hey there".to_string()),
        )
        .await?;
    println!("called message and got response {:?}", response);
    Ok(())
}
