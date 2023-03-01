// pub use casper_client;
// pub use casper_node;
// pub use casper_types;
pub mod tls;

use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Cursor, Write},
    path::{Path, PathBuf},
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
/// Prod:
/// service
///     casper-node-launcher
///         casper-node
/// service
///     casper-node-launcher
///         casper-node(versioned) -> wrapper to hook debugging tools
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
    /// Start a service with the given parameters on the host running the agent.
    async fn start_service(request: StartServiceRequest) -> StartServiceResponse;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentUpdateRequest {
    new_version: u32,
    dist_tarball: CompressedWireFile,
}

#[derive(Debug, Serialize, Deserialize, StructOpt)]
pub enum AgentUpdateResponse {
    Success {
        old_version: u32,
        new_version: u32,
        new_pid: u16,
    },
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
    pub host_src_path: PathBuf,
    pub filename: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FetchFileResponse {
    Success { file: CompressedWireFile },
    Error,
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
        src_path: &Path,
        target_path: &Path,
    ) -> Result<Self, MessageError> {
        Ok(Self {
            target_perms: 0o666,
            target_path: target_path.to_path_buf(),
            file: CompressedWireFile::load_and_compress(src_path, target_path)?,
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
    pub fn load_and_compress(src_path: &Path, target_path: &Path) -> Result<Self, MessageError> {
        let file = File::open(src_path).map_err(|err| MessageError::OpenFile {
            path: src_path.to_path_buf(),
            err,
        })?;
        let filename = file_name_from_path(target_path)?;
        let reader = BufReader::new(file);
        let zstd_compressed_data =
            zstd::encode_all(reader, 3).map_err(|err| MessageError::Compress {
                path: src_path.to_path_buf(),
                err,
            })?;
        Ok(CompressedWireFile {
            filename,
            zstd_compressed_data,
        })
    }

    /// Decompresses and then writes a compressed file message to disk as the file it represents.
    /// Assumes the directory it's writing into exists.
    pub fn into_file_on_disk(self, destination_path: &PathBuf) -> Result<(), std::io::Error> {
        let mut data = Cursor::new(self.zstd_compressed_data);
        let file = File::create(destination_path)?;
        let mut decoder = zstd::Decoder::new(&mut data)?;
        let mut writer = BufWriter::new(file);
        std::io::copy(&mut decoder, &mut writer)?;
        writer.flush()?;
        Ok(())
    }
    /// On the agent side, deserialized but needs to be put to disk.
    pub fn into_temp_file_on_disk(self) -> Result<PathBuf, std::io::Error> {
        let target_temp_path = PathBuf::from("./temp");
        fs::create_dir_all(&target_temp_path)?;
        let target_file = target_temp_path.join(&self.filename);
        self.into_file_on_disk(&target_file)?;
        Ok(target_temp_path)
    }
}

pub fn file_name_from_path(target_path: &Path) -> Result<String, MessageError> {
    let filename = target_path
        .file_name()
        .and_then(|os_str| os_str.to_str())
        .ok_or_else(|| MessageError::NoFileName)?
        .to_string();
    Ok(filename)
}

#[cfg(test)]
mod tests {
    #[test]
    fn round_trip_compress_decompress() {}
}
