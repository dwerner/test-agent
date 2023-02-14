// pub use casper_client;
// pub use casper_node;
// pub use casper_types;
pub mod pkg_manager;
mod tls;

use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

use pin_project::pin_project;
use structopt::StructOpt;
use tarpc::tokio_serde::{Deserializer, Serializer};
use tarpc::tokio_util::codec::{length_delimited, LengthDelimitedCodec};
use tarpc::ClientMessage;
use tarpc::{
    serde::{Deserialize, Serialize},
    tokio_serde::formats::Bincode,
};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio_rustls::{TlsAcceptor, TlsConnector, TlsStream};

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
///
/// Needless to say, but this service is designed to be used in a debug environment
#[tarpc::service]
pub trait AgentService {
    async fn install_package(request: InstallPackageRequest) -> InstallPackageResponse;
    async fn start_node(request: StartNodeRequest) -> StartNodeResponse;
}

// async fn connect_tls(domain: &str, port: u16) -> Result<TlsStream<TcpStream>, std::io::Error> {
//     let mut roots = rustls::RootCertStore::empty();
//     for cert in rustls_native_certs::load_native_certs().expect("could not load os certificates") {
//         roots.add(&rustls::Certificate(cert.0)).unwrap();
//     }
//     let config = rustls::ClientConfig::builder()
//         .with_safe_defaults()
//         .with_root_certificates(roots)
//         .with_no_client_auth();
//     let connector = TlsConnector::from(Arc::new(config));
//     let servername = rustls::ServerName::try_from(domain).unwrap();

//     let host = format!("{}:{}", domain, port);
//     let stream = TcpStream::connect(host).await?;
//     connector.connect(servername, stream).await
// }

async fn serve_tls(
    domain: &str,
    port: u16,
    cert_file: &str,
    key_file: &str,
) -> Result<TlsAcceptor, anyhow::Error> {
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().expect("could not load os certificates") {
        roots.add(&rustls::Certificate(cert.0)).unwrap();
    }

    let cert = {
        let mut cert_data = vec![];
        let mut cert_file = File::open(cert_file)?;
        cert_file.read_to_end(&mut cert_data)?;
        rustls::Certificate(cert_data)
    };
    let key = {
        let mut key_data = vec![];
        let mut key_file = File::open(key_file)?;
        key_file.read_to_end(&mut key_data)?;
        rustls::PrivateKey(key_data)
    };
    // TODO: add self-signed cert
    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        //.with_ca_certificates(roots)
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)?;

    let server_addr = (IpAddr::V6(Ipv6Addr::LOCALHOST), 8081);
    let mut listener = tls::listen::<
        ClientMessage<AgentServiceRequest>,
        ClientMessage<AgentServiceResponse>,
        Bincode<_, _>,
        fn() -> Bincode<_, _>,
    >(&server_addr, tarpc::tokio_serde::formats::Bincode::default)
    .await?;

    listener
        .config_mut()
        .max_frame_length(std::u32::MAX as usize);

    let acceptor = TlsAcceptor::from(Arc::new(config));

    Ok(acceptor)
}
