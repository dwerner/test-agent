// pub use casper_client;
// pub use casper_node;
// pub use casper_types;
pub mod pkg_manager;
pub mod tls;

use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Cursor, Write},
    path::PathBuf,
};
use structopt::StructOpt;

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
    /// Push a file to the host running the agent.
    async fn put_file(req: PutFileRequest) -> PutFileResponse;
    /// Fetch a file from the host running the agent.
    async fn fetch_file(req: FetchFileRequest) -> FetchFileResponse;
    /// Stop a service with the given parameters on the host running the agent.
    async fn stop_service(request: StartServiceRequest) -> StartServiceResponse;
    /// Install a package on the host running the agent using it's package manager.
    async fn install_package(request: InstallPackageRequest) -> InstallPackageResponse;
    /// Start a service with the given parameters on the host running the agent.
    async fn start_service(request: StartServiceRequest) -> StartServiceResponse;
}

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

#[derive(thiserror::Error, Debug)]
pub enum MessageError {
    #[error("file path provided has no 'filename'.")]
    NoFileName,
    #[error("file {path} could not be opened {err:?}")]
    OpenFile { path: PathBuf, err: std::io::Error },
    #[error("file {path} could not be read {err:?}")]
    ReadFile { path: PathBuf, err: std::io::Error },
    #[error("error compressing data from {path} - {err:?}")]
    Compress { path: PathBuf, err: std::io::Error },
}

/// Cannot be constructed directly from the commandline.
#[derive(Debug, Serialize, Deserialize)]
pub struct PutFileRequest {
    pub target_perms: u32,
    pub target_path: PathBuf,
    pub file: CompressedWireFile,
}

impl PutFileRequest {
    /// Loads a file at the given src_path, compresses it's contents using zstd and creates a message containing the compressed data.
    pub fn new_with_default_perms(
        src_path: &PathBuf,
        target_path: &PathBuf,
    ) -> Result<Self, MessageError> {
        let file = File::open(src_path).map_err(|err| MessageError::OpenFile {
            path: src_path.clone(),
            err,
        })?;

        let filename = target_path
            .file_name()
            .map(|os_str| os_str.to_str())
            .flatten()
            .ok_or_else(|| MessageError::NoFileName)?
            .to_string();

        let reader = BufReader::new(file);

        let zstd_compressed_data =
            zstd::encode_all(reader, 3).map_err(|err| MessageError::Compress {
                path: src_path.clone(),
                err,
            })?;

        Ok(Self {
            target_perms: 0,
            target_path: target_path.clone(),
            file: CompressedWireFile {
                filename,
                zstd_compressed_data,
            },
        })
    }
}

#[derive(Debug, Serialize, Deserialize, StructOpt)]
pub enum PutFileResponse {
    Success,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompressedWireFile {
    pub filename: String,
    pub zstd_compressed_data: Vec<u8>,
}

impl CompressedWireFile {
    /// On the agent side, deserialized but needs to be put to disk.
    pub fn into_temp_file_on_disk(self) -> Result<(File, PathBuf), std::io::Error> {
        let mut data = Cursor::new(self.zstd_compressed_data);
        let mut target_temp_path = PathBuf::from("./temp");
        fs::create_dir_all(&target_temp_path)?;
        target_temp_path.push(self.filename);
        let file = File::create(&target_temp_path)?;
        let mut decoder = zstd::Decoder::new(&mut data)?;
        let mut writer = BufWriter::new(file);
        std::io::copy(&mut decoder, &mut writer)?;
        writer.flush()?;
        let file = writer.into_inner()?;
        Ok((file, target_temp_path))
    }
}
