use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc, time::Instant};

use agent_lib::{
    tls, AgentService, CompressedWireFile, CompressedWireFileChunk, FetchFileRequest,
    FetchFileResponse, PutFileChunkRequest, PutFileChunkResponse, PutFileRequest, PutFileResponse,
    StartServiceRequest, StartServiceResponse,
};
use async_mutex::Mutex;
use futures::{future, StreamExt};
use structopt::StructOpt;
use tarpc::{
    context::Context,
    server::{self, incoming::Incoming, Channel},
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
        .max_channels_per_key(1, |t| t.transport().peer_addr().unwrap().ip())
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
    in_flight_transfers: Arc<Mutex<HashMap<[u8; 32], InFlightTransfer>>>,
}

#[derive(Debug, Clone)]
struct InFlightTransfer {
    target_path: PathBuf,
    target_perms: u32,
    last_updated: Instant,
    chunks: Vec<CompressedWireFileChunk>,
}

impl Agent {
    fn new(addr: SocketAddr) -> Result<Self, AgentError> {
        Ok(Self {
            _addr: addr,
            in_flight_transfers: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

#[tarpc::server]
impl AgentService for Agent {
    async fn put_file_chunk(self, _: Context, req: PutFileChunkRequest) -> PutFileChunkResponse {
        let PutFileChunkRequest {
            file_hash,
            target_perms,
            target_path,
            chunk,
        } = req;
        let chunk_id = chunk.chunk_id;
        let complete_transfer = {
            let mut lock = self.in_flight_transfers.lock().await;
            {
                let transfer = lock.entry(file_hash).or_insert_with(|| InFlightTransfer {
                    last_updated: Instant::now(),
                    target_path,
                    target_perms,
                    chunks: Vec::new(),
                });
                if transfer
                    .chunks
                    .iter()
                    .find(|c| c.chunk_id == chunk_id)
                    .is_some()
                {
                    println!("already have chunk with id {chunk_id}");
                    return PutFileChunkResponse::Duplicate { chunk_id };
                }
                transfer.last_updated = Instant::now();
                transfer.chunks.push(chunk);

                if transfer.chunks.len() == transfer.chunks[0].num_chunks as usize {
                    lock.remove(&file_hash).expect("transfer must exist")
                } else {
                    return PutFileChunkResponse::Progress {
                        chunk_id,
                        seen_chunks: transfer.chunks.len() as u64,
                    };
                }
            }
        };

        match CompressedWireFile::from_chunks(complete_transfer.chunks) {
            Ok(file) => {
                let b3_hash = file.blake3_hash();
                if b3_hash != file_hash {
                    println!("file hash mismatch - expected {file_hash:x?} got {b3_hash:x?}");
                    return PutFileChunkResponse::Error { chunk_id };
                }
                let temp_path = match file.into_temp_file_on_disk() {
                    Ok(temp_path) => temp_path,
                    Err(err) => {
                        println!("err while assembling file from chunks {err:?}");
                        return PutFileChunkResponse::Error { chunk_id };
                    }
                };

                println!("do more than copy file to temp dir - this needs to implement the copy to dest as well.");
                println!(
                    "would write to disk: {}, with perms {target_perms} temp file in {}",
                    temp_path.display(),
                    complete_transfer.target_path.display()
                );
            }
            Err(err) => {
                println!("err while assembling file from chunks {err:?}");
                return PutFileChunkResponse::Error { chunk_id };
            }
        }

        PutFileChunkResponse::Complete { chunk_id }
    }

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
        match CompressedWireFile::load_and_compress(&host_src_path, &filename) {
            Ok(file) => FetchFileResponse::Success { file },
            Err(err) => {
                println!("err while loading file for fetching {err:?}");
                FetchFileResponse::Error
            }
        }
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
