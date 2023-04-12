use std::{
    fs::{self, File},
    io::{BufReader, Read},
    net::SocketAddr,
    path::PathBuf,
    str::FromStr,
};

use agent_lib::{
    file_name_from_path, tls, AgentServiceClient, FetchFileRequest, FetchFileResponse,
    PutFileRequest, StartServiceRequest, StopServiceRequest,
};
use serde::Deserialize;
use structopt::StructOpt;
use tarpc::{client, context, tokio_serde::formats::Bincode};

#[derive(Debug, structopt::StructOpt)]
struct Args {
    #[structopt(short)]
    daemon_peers: Option<Peers>,
    #[structopt(long, default_value = "assets/agent-crt.pem")]
    cert: PathBuf,
    #[structopt(long, default_value = "assets/agent-key.pem")]
    key: PathBuf,
    #[structopt(subcommand)]
    rpc: Rpc,
}

#[derive(Clone, Debug, structopt::StructOpt)]
enum Rpc {
    StartService(StartServiceRequest),
    StopService(StopServiceRequest),
    FetchFile(FetchFileRequest),
    PutFile(PutFile),
    PutFileChunked(PutFile),
}

#[derive(Debug, structopt::StructOpt, Deserialize)]
enum Peers {
    /// List of peers, comma separated from the cmdline
    List(PeersList),
    Yaml {
        path: PathBuf,
    },
}

#[derive(Debug, structopt::StructOpt, Deserialize)]
struct PeersList {
    pub peers: Vec<SocketAddr>,
}

impl FromStr for Peers {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let peers = match s
            .split(',')
            .map(|s| s.parse::<SocketAddr>())
            .collect::<Result<Vec<SocketAddr>, _>>()
        {
            Ok(peers) => Peers::List(PeersList { peers }),
            Err(_err) => {
                let path = PathBuf::from(s);
                Peers::Yaml { path }
            }
        };
        Ok(peers)
    }
}

#[derive(Clone, Debug, StructOpt)]
pub struct PutFile {
    source_file: PathBuf,
    target_path: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts: Args = Args::from_args();

    let peers = match opts.daemon_peers {
        Some(Peers::List(peers)) => peers,
        Some(Peers::Yaml { path }) => {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            let peers: PeersList = serde_yaml::from_reader(reader)?;
            peers
        }
        None => {
            println!("no peers specified");
            return Ok(());
        }
    };

    println!("using peers {:?}", peers);

    let mut clients = Vec::new();
    for peer in peers.peers.iter() {
        println!("connecting to {}", peer);
        let tls = tls::connect(peer, &opts.cert, &opts.key).await.unwrap();
        let transport = tarpc::serde_transport::Transport::from((tls, Bincode::default()));
        let client = AgentServiceClient::new(client::Config::default(), transport).spawn();
        clients.push(client);
    }

    let mut responses = Vec::new();
    for client in clients {
        let rpc = opts.rpc.clone();
        let response_future = async move {
            match rpc {
                Rpc::StopService(_stop) => todo!(),
                Rpc::FetchFile(fetch) => {
                    let filename = file_name_from_path(&fetch.filename).unwrap();
                    let response = client.fetch_file(context::current(), fetch).await?;
                    fs::create_dir_all("./fetch")?;
                    if let FetchFileResponse::Success { file } = response {
                        let target_path = PathBuf::from(format!("./fetch/{}", filename));
                        file.into_file_on_disk(&target_path).unwrap();
                        println!("fetch file succeeded. TODO FILE SIZES, times?");
                    } else {
                        println!("fetch file failed");
                        todo!()
                    }
                }
                Rpc::PutFileChunked(put) => {
                    let req =
                        PutFileRequest::new_with_default_perms(&put.source_file, &put.target_path)?;
                    let chunks = req.into_chunked_requests(5242880);
                    for chunked_req in chunks.into_iter() {
                        println!("chunked put file request: {chunked_req:?}");
                        let response = client
                            .put_file_chunk(context::current(), chunked_req)
                            .await?;
                        println!("chunked put file response: {response:?}");
                    }
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
            }
            Ok::<(), anyhow::Error>(())
        };
        responses.push(response_future);
    }

    futures::future::join_all(responses).await;
    Ok(())
}
