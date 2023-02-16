// pub use casper_client;
// pub use casper_node;
// pub use casper_types;
pub mod pkg_manager;
pub mod tls;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(Debug, Serialize, Deserialize, StructOpt)]
pub struct InstallPackageRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, StructOpt)]
pub enum InstallPackageResponse {
    Success,
    AlreadyInstalled,
    Error,
}

#[derive(Debug, Serialize, Deserialize, StructOpt)]
pub struct StartServiceRequest {
    // TODO something like a wrapper over systemd, casper-updater, and extended to support other things like heaptrack, valgrind, etc
    pub wrapper: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, StructOpt)]
pub enum StartServiceResponse {
    Success,
    Restarted,
    Error,
}

/// The responsibilities of the Agent are to:
/// - install required software on the given target
/// - install assets required for the casper-node-launcher and casper-node to run
/// - install and stage upgrades
/// Software install strategy:
///     - can install software on ubuntu and arch itself, through elevated privileges.
///     - can be fed a binary directly
///
/// - start the node launcher
/// - stop the given node launcher
///
/// - find the node process
/// - determine mapped ports
/// - restart the node with a particular wrapper:
///     - gdb
///     - valgrind
///     - perf
///     - heaptrack
/// - deliver artifacts of those actions via a zstd compressed interface
///
/// Needless to say, but this service is designed to be used in a debug environment
#[tarpc::service]
pub trait AgentService {
    async fn put_file(req: PutFileRequest) -> PutFileResponse;
    async fn fetch_file(req: FetchFileRequest) -> FetchFileResponse;
    async fn stop_service(request: StartServiceRequest) -> StartServiceResponse;

    async fn install_package(request: InstallPackageRequest) -> InstallPackageResponse;
    async fn start_service(request: StartServiceRequest) -> StartServiceResponse;
}

#[derive(Debug, Serialize, Deserialize, StructOpt)]
pub struct FetchFileRequest {
    pub target_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FetchFileResponse {
    Success { file: CompressedWireFile },
}

#[derive(Debug, Serialize, Deserialize, StructOpt)]
pub struct StopServiceRequest {
    service: String,
}

#[derive(Debug, Serialize, Deserialize, StructOpt)]
pub enum StopServiceResponse {
    Success,
    Restarted,
    Error,
}

/// Cannot be constructed directly from the commandline.
#[derive(Debug, Serialize, Deserialize)]
pub struct PutFileRequest {
    pub target_path: PathBuf,
    pub file: CompressedWireFile,
}

#[derive(Debug, Serialize, Deserialize, StructOpt)]
pub enum PutFileResponse {
    Success,
    Error,
}

#[derive(Debug, Serialize, Deserialize, StructOpt)]
pub struct CompressedWireFile {
    pub filename: String,
    pub zstd_compressed_data: Vec<u8>,
}
