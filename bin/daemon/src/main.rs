use std::{net::SocketAddr, path::PathBuf};

use agent_lib::{
    tls, AgentService, CompressedWireFile, FetchFileRequest, FetchFileResponse, PutFileRequest,
    PutFileResponse, StartServiceRequest, StartServiceResponse,
};
use futures::{future, StreamExt};
use structopt::StructOpt;
use tarpc::{
    context::Context,
    server::{self, Channel},
    tokio_serde::formats::Bincode,
};

#[derive(Debug, StructOpt)]
enum Args {
    Serve {
        #[structopt(default_value = "0.0.0.0:8081")]
        addr: SocketAddr,
        #[structopt(default_value = "assets/agent-crt.pem")]
        cert: PathBuf,
        #[structopt(default_value = "assets/agent-key.pem")]
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
        .for_each(|_| async {})
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
    async fn put_file(self, _ctx: Context, req: PutFileRequest) -> PutFileResponse {
        let PutFileRequest {
            target_path,
            target_perms,
            file,
        } = req;

        let temp_path = file
            .into_temp_file_on_disk()
            .expect("TODO - unable to write temp file");

        println!("do more than copy file to temp dir - this needs to implement the copy to dest as well.");
        println!(
            "would write to disk: {}, with perms {target_perms} temp file in {}",
            temp_path.display(),
            target_path.display()
        );
        PutFileResponse::Success
    }

    async fn fetch_file(self, _ctx: Context, req: FetchFileRequest) -> FetchFileResponse {
        let FetchFileRequest {
            host_src_path,
            filename,
        } = req;
        let response = match CompressedWireFile::load_and_compress(&host_src_path, &filename) {
            Ok(file) => FetchFileResponse::Success { file },
            Err(err) => {
                println!("err while loading file for fetching {err:?}");
                FetchFileResponse::Error
            }
        };
        response
    }

    async fn stop_service(
        self,
        _ctx: Context,
        _request: StartServiceRequest,
    ) -> StartServiceResponse {
        todo!()
    }

    async fn start_service(
        self,
        _: Context,
        _request: StartServiceRequest,
    ) -> StartServiceResponse {
        for i in 0..10 {
            println!("did some work {i}");
        }
        StartServiceResponse::Error
    }
}
