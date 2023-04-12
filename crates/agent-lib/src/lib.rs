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
    /// Transfer a chunk of a file to the host running the agent.
    async fn put_file_chunk(chunk: PutFileChunkRequest) -> PutFileChunkResponse;
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

#[derive(Clone, Debug, Serialize, Deserialize, StructOpt)]
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

#[derive(Clone, Debug, Serialize, Deserialize, StructOpt)]
pub struct FetchFileRequest {
    pub host_src_path: PathBuf,
    pub filename: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FetchFileResponse {
    Success { file: CompressedWireFile },
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize, StructOpt)]
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
    #[error("no chunks provided")]
    NoChunks,
    #[error("wrong number of chunks provided, expected {expected}, got {actual}")]
    WrongNumberOfChunks { expected: usize, actual: usize },
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

    /// Loads a file at the given src_path, compresses it's contents using zstd and creates a message containing the compressed data.
    pub fn into_chunked_requests(
        &self,
        chunk_size: usize,
    ) -> impl Iterator<Item = PutFileChunkRequest> + '_ {
        let target_perms = self.target_perms;
        let target_path = &self.target_path;
        let file_hash = self.file.blake3_hash();
        self.file
            .into_chunks_with_size(chunk_size)
            .map(move |chunk| PutFileChunkRequest {
                file_hash,
                target_perms,
                target_path: target_path.clone(),
                chunk,
            })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PutFileChunkRequest {
    pub file_hash: [u8; 32],
    pub target_perms: u32,
    pub target_path: PathBuf,
    pub chunk: CompressedWireFileChunk,
}

impl PutFileChunkRequest {
    pub fn new(
        file_hash: [u8; 32],
        target_perms: u32,
        target_path: PathBuf,
        chunk: CompressedWireFileChunk,
    ) -> Self {
        Self {
            file_hash,
            target_perms,
            target_path,
            chunk,
        }
    }
}

/// Put a file chunk on the host running the agent.
#[derive(Debug, Serialize, Deserialize, StructOpt)]
pub enum PutFileChunkResponse {
    Complete { chunk_id: u64 },
    Progress { chunk_id: u64, seen_chunks: u64 },
    Error { chunk_id: u64 },
    Duplicate { chunk_id: u64 },
}

#[derive(Debug, Serialize, Deserialize, StructOpt)]
pub enum PutFileResponse {
    Success,
    Error,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CompressedWireFileChunk {
    pub filename: String,
    pub chunk_id: u64,
    pub num_chunks: u64,
    pub zstd_compressed_data_chunk: Vec<u8>,
}

impl std::fmt::Debug for CompressedWireFileChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompressedWireFileChunk")
            .field("filename", &self.filename)
            .field("chunk_id", &self.chunk_id)
            .field("num_chunks", &self.num_chunks)
            .field(
                "zstd_compressed_data_chunk",
                &self.zstd_compressed_data_chunk.len(),
            )
            .finish()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompressedWireFile {
    pub filename: String,
    pub zstd_compressed_data: Vec<u8>,
}

impl CompressedWireFile {
    /// Generate a blake3 hash of the compressed data.
    pub fn blake3_hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.zstd_compressed_data);
        hasher.finalize().into()
    }

    /// Build a file from a list of chunks.
    pub fn from_chunks(mut chunks: Vec<CompressedWireFileChunk>) -> Result<Self, MessageError> {
        let mut zstd_compressed_data = Vec::new();

        chunks.sort_by_key(|chunk| chunk.chunk_id);
        if chunks.is_empty() {
            return Err(MessageError::NoChunks);
        }
        if chunks.len() != chunks[0].num_chunks as usize {
            return Err(MessageError::WrongNumberOfChunks {
                expected: chunks[0].num_chunks as usize,
                actual: chunks.len(),
            });
        }

        for chunk in chunks.iter() {
            zstd_compressed_data.extend_from_slice(&chunk.zstd_compressed_data_chunk);
        }

        Ok(Self {
            filename: chunks[0].filename.clone(),
            zstd_compressed_data,
        })
    }

    /// Turn a loaded file into a set of chunks for transmission.
    pub fn into_chunks_with_size(
        &self,
        chunk_size: usize,
    ) -> impl Iterator<Item = CompressedWireFileChunk> + '_ {
        let filename = &self.filename;
        let zstd_compressed_data = &self.zstd_compressed_data;
        let num_chunks = (zstd_compressed_data.len() + chunk_size - 1) / chunk_size;
        let chunks = zstd_compressed_data.chunks(chunk_size);

        chunks
            .enumerate()
            .map(move |(chunk_id, chunk)| CompressedWireFileChunk {
                filename: filename.clone(),
                chunk_id: chunk_id as u64,
                num_chunks: num_chunks as u64,
                zstd_compressed_data_chunk: chunk.to_vec(),
            })
    }

    /// Load a file and compress it in memory.
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
    use super::*;

    #[test]
    fn test_chunking_and_reassembling() {
        // Create a sample CompressedWireFile
        let filename = "test.txt".to_string();
        let zstd_compressed_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let compressed_wire_file = CompressedWireFile {
            filename: filename.clone(),
            zstd_compressed_data: zstd_compressed_data.clone(),
        };

        // Define the desired chunk size
        let chunk_size = 4;

        // Chunk the CompressedWireFile
        let chunks: Vec<CompressedWireFileChunk> = compressed_wire_file
            .into_chunks_with_size(chunk_size)
            .collect();

        // Verify the number of chunks is correct
        let expected_num_chunks =
            (zstd_compressed_data.len() as f64 / chunk_size as f64).ceil() as u64;
        assert_eq!(chunks.len(), expected_num_chunks as usize);

        // Verify the chunk content and metadata
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.filename, filename);
            assert_eq!(chunk.chunk_id, i as u64);
            assert_eq!(chunk.num_chunks, expected_num_chunks);

            let start = i * chunk_size;
            let end = usize::min(start + chunk_size, zstd_compressed_data.len());
            let expected_data = &zstd_compressed_data[start..end];
            assert_eq!(chunk.zstd_compressed_data_chunk, expected_data);
        }

        // Reassemble the CompressedWireFile from the chunks
        let reassembled_compressed_wire_file = CompressedWireFile::from_chunks(chunks).unwrap();

        // Verify the reassembled CompressedWireFile is identical to the original
        assert_eq!(reassembled_compressed_wire_file.filename, filename);
        assert_eq!(
            reassembled_compressed_wire_file.zstd_compressed_data,
            zstd_compressed_data
        );
    }
}
