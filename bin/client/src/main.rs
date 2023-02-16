use std::{fs, net::SocketAddr, path::PathBuf};

use agent_lib::{
    file_name_from_path, tls, AgentServiceClient, FetchFileRequest, FetchFileResponse,
    InstallPackageRequest, PutFileRequest, StartServiceRequest, StopServiceRequest,
};
use structopt::StructOpt;
use tarpc::{client, context, tokio_serde::formats::Bincode};

#[derive(Debug, structopt::StructOpt)]
struct Args {
    #[structopt(default_value = "0.0.0.0:8081")]
    bind_addr: SocketAddr,
    #[structopt(default_value = "assets/agent-crt.pem")]
    cert: PathBuf,
    #[structopt(default_value = "assets/agent-key.pem")]
    key: PathBuf,
    #[structopt(subcommand)]
    rpc: Rpc,
}

#[derive(Debug, structopt::StructOpt)]
enum Rpc {
    StartService(StartServiceRequest),
    StopService(StopServiceRequest),
    FetchFile(FetchFileRequest),
    PutFile(PutFile),
    InstallPackage(InstallPackageRequest),
}

#[derive(Debug, StructOpt)]
pub struct PutFile {
    source_file: PathBuf,
    target_path: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts: Args = structopt::StructOpt::from_args();
    // println!("opts {:?}", opts);
    let tls = tls::connect(&opts.bind_addr, &opts.cert, &opts.key)
        .await
        .unwrap();
    let transport = tarpc::serde_transport::Transport::from((tls, Bincode::default()));
    let client = AgentServiceClient::new(client::Config::default(), transport).spawn();

    match opts.rpc {
        Rpc::StopService(_stop) => todo!(),
        Rpc::FetchFile(fetch) => {
            let filename = file_name_from_path(&fetch.filename).unwrap();
            let response = client.fetch_file(context::current(), fetch).await?;
            fs::create_dir_all("./fetch")?;
            if let FetchFileResponse::Success { file } = response {
                let target_path = PathBuf::from(format!("./fetch/{}", filename));
                file.into_file_on_disk(&target_path).unwrap();
                println!("fetch file succeeded. TODO FILE SIZES, times?");
                return Ok(());
            }
            println!("fetch file failed");
            todo!()
        }
        Rpc::PutFile(put) => {
            let put_file_request =
                PutFileRequest::new_with_default_perms(&put.source_file, &put.target_path)?;
            let response = client
                .put_file(context::current(), put_file_request)
                .await?;

            println!("put file response: {response:?}");
        }

        Rpc::StartService(start) => {
            let response = client.start_service(context::current(), start).await?;
            println!("called start and got response {response:?}");
        }
        Rpc::InstallPackage(install) => {
            let response = client.install_package(context::current(), install).await?;
            println!("called install package and got response {response:?}");
        }
    }
    Ok(())
}
