// pub use casper_client;
// pub use casper_node;
// pub use casper_types;
pub mod pkg_manager;

use structopt::StructOpt;
use tarpc::serde::{Deserialize, Serialize};

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
pub struct StartNodeRequest {
    // TODO something like a wrapper over systemd, casper-updater, and extended to support other things like heaptrack, valgrind, etc
    pub wrapper: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, StructOpt)]
pub enum StartNodeResponse {
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
#[tarpc::service]
pub trait AgentService {
    async fn install_package(request: InstallPackageRequest) -> InstallPackageResponse;
    async fn start_node(request: StartNodeRequest) -> StartNodeResponse;
}
