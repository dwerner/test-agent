use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::task::Waker;
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use std::{marker::PhantomData, net::SocketAddr};

use futures::{ready, Sink};
use futures::{Future, Stream};
use pin_project::pin_project;
use rustls::client::ServerCertVerifier;
use rustls::ServerConfig;
use rustls_pemfile::Item;
use serde::{Deserialize, Serialize};
use tarpc::serde_transport::Transport as TarpcTransport;
use tarpc::tokio_serde::{Deserializer, Serializer};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::server::TlsStream;
use tokio_rustls::{client, Accept, TlsAcceptor, TlsConnector};
use tokio_serde::Framed as SerdeFramed;
use tokio_util::codec::length_delimited;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

/// Constructs a new transport from a framed transport and a serialization codec.
pub fn new<Item, SinkItem, Codec>(
    framed_io: Framed<TlsStream<TcpStream>, LengthDelimitedCodec>,
    codec: Codec,
) -> Transport<Item, SinkItem, Codec>
where
    Item: for<'de> Deserialize<'de>,
    SinkItem: Serialize,
    Codec: Serializer<SinkItem> + Deserializer<Item>,
{
    Transport {
        inner: tarpc::serde_transport::new(framed_io, codec),
    }
}

#[pin_project]
pub struct Transport<Item, SinkItem, Codec> {
    #[pin]
    inner: TarpcTransport<TlsStream<TcpStream>, Item, SinkItem, Codec>,
}

impl<Item, SinkItem, Codec, CodecError> Stream for Transport<Item, SinkItem, Codec>
where
    Item: for<'a> Deserialize<'a>,
    Codec: Deserializer<Item>,
    CodecError: Into<Box<dyn std::error::Error + Send + Sync>>,
    SerdeFramed<Framed<TlsStream<TcpStream>, LengthDelimitedCodec>, Item, SinkItem, Codec>:
        Stream<Item = Result<Item, CodecError>>,
{
    type Item = io::Result<Item>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<io::Result<Item>>> {
        self.project()
            .inner
            .poll_next(cx)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}

impl<Item, SinkItem, Codec, CodecError> Sink<SinkItem> for Transport<Item, SinkItem, Codec>
where
    SinkItem: Serialize,
    Codec: Serializer<SinkItem>,
    CodecError: Into<Box<dyn Error + Send + Sync>>,
    SerdeFramed<Framed<TlsStream<TcpStream>, LengthDelimitedCodec>, Item, SinkItem, Codec>:
        Sink<SinkItem, Error = CodecError>,
{
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project()
            .inner
            .poll_ready(cx)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn start_send(self: Pin<&mut Self>, item: SinkItem) -> io::Result<()> {
        self.project()
            .inner
            .start_send(item)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project()
            .inner
            .poll_flush(cx)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project()
            .inner
            .poll_close(cx)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}

impl<Item, SinkItem, Codec> Transport<Item, SinkItem, Codec> {
    /// Returns the peer address of the underlying TcpStream.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.inner.get_ref().get_ref().0.peer_addr()
    }
    /// Returns the local address of the underlying TcpStream.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.get_ref().get_ref().0.local_addr()
    }
}

/// Listens on `addr`, wrapping accepted connections in TCP transports.
pub async fn listen<Item, SinkItem, Codec, CodecFn>(
    addr: &SocketAddr,
    config: ServerConfig,
    codec_fn: CodecFn,
) -> io::Result<TlsIncoming<Item, SinkItem, Codec, CodecFn>>
where
    Item: for<'de> Deserialize<'de>,
    Codec: Serializer<SinkItem> + Deserializer<Item>,
    CodecFn: Fn() -> Codec,
{
    println!("serving tls connections on {addr}");
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    Ok(TlsIncoming {
        acceptor,
        accept: None,
        waker: None,
        listener,
        codec_fn,
        local_addr,
        config: LengthDelimitedCodec::builder(),
        ghost: PhantomData,
    })
}

/// A [`TcpListener`] that wraps connections in [transports](Transport).
#[allow(clippy::type_complexity)]
#[pin_project]
pub struct TlsIncoming<Item, SinkItem, Codec, CodecFn> {
    acceptor: TlsAcceptor,
    #[pin]
    accept: Option<Accept<TcpStream>>,
    #[pin]
    waker: Option<Waker>,
    listener: TcpListener,
    local_addr: SocketAddr,
    codec_fn: CodecFn,
    config: length_delimited::Builder,
    ghost: PhantomData<(fn() -> Item, fn(SinkItem), Codec)>,
}

impl<Item, SinkItem, Codec, CodecFn> TlsIncoming<Item, SinkItem, Codec, CodecFn> {
    /// Returns the address being listened on.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Returns an immutable reference to the length-delimited codec's config.
    pub fn config(&self) -> &length_delimited::Builder {
        &self.config
    }

    /// Returns a mutable reference to the length-delimited codec's config.
    pub fn config_mut(&mut self) -> &mut length_delimited::Builder {
        &mut self.config
    }
}

impl<Item, SinkItem, Codec, CodecFn> Stream for TlsIncoming<Item, SinkItem, Codec, CodecFn>
where
    Item: for<'de> Deserialize<'de>,
    SinkItem: Serialize,
    Codec: Serializer<SinkItem> + Deserializer<Item>,
    CodecFn: Fn() -> Codec,
{
    type Item = io::Result<Transport<Item, SinkItem, Codec>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match self.accept.as_mut() {
            None => {
                let conn: TcpStream =
                    ready!(Pin::new(&mut self.as_mut().project().listener).poll_accept(cx)?).0;
                self.accept = Some(self.acceptor.accept(conn));
                let waker = cx.waker().clone();
                waker.wake_by_ref();
                self.waker = Some(waker);
                Poll::Pending
            }
            Some(mut accept) => match Pin::new(&mut accept).poll(cx) {
                Poll::Ready(tls) => {
                    self.waker.take();
                    self.accept.take();
                    match tls {
                        Ok(tls) => Poll::Ready(Some(Ok(new(
                            self.config.new_framed(tls),
                            (self.codec_fn)(),
                        )))),
                        Err(err) => Poll::Ready(Some(Err(err))),
                    }
                }
                Poll::Pending => Poll::Pending,
            },
        }
    }
}

pub async fn serve<I, SinkItem, Codec, CodecFn>(
    addr: SocketAddr,
    cert_file: PathBuf,
    key_file: PathBuf,
    codec_fn: CodecFn,
) -> Result<TlsIncoming<I, SinkItem, Codec, CodecFn>, anyhow::Error>
where
    I: for<'de> Deserialize<'de>,
    Codec: Serializer<SinkItem> + Deserializer<I>,
    CodecFn: Fn() -> Codec,
{
    let key = load_key(&key_file)?;
    let cert = load_cert(&cert_file)?;

    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)?;

    let mut listener = listen::<I, SinkItem, Codec, CodecFn>(&addr, config, codec_fn).await?;

    listener
        .config_mut()
        .max_frame_length(std::u32::MAX as usize);

    Ok(listener)
}

pub async fn connect(
    addr: &SocketAddr,
    cert_file: &Path,
    key_file: &Path,
) -> Result<client::TlsStream<TcpStream>, anyhow::Error> {
    let mut roots = rustls::RootCertStore::empty();

    // only valid for self signed certs, which is what we have.
    let end_entity = load_cert(cert_file)?;
    let key = load_key(key_file)?;

    roots.add(&end_entity)?;
    let mut config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_single_cert(vec![end_entity.clone()], key)?;

    config.dangerous().set_certificate_verifier(
        Arc::new(SelfSignedCertResolver { end_entity }) as Arc<dyn ServerCertVerifier>
    );

    let connector = TlsConnector::from(Arc::new(config));
    let stream = TcpStream::connect(addr).await?;
    Ok(connector
        .connect(rustls::ServerName::IpAddress(addr.ip()), stream)
        .await?)
}

fn load_key(key_file: &Path) -> Result<rustls::PrivateKey, anyhow::Error> {
    let mut reader = BufReader::new(File::open(key_file)?);
    Ok(rustls::PrivateKey(
        match rustls_pemfile::read_one(&mut reader)? {
            Some(Item::PKCS8Key(cert)) => cert,
            other => return Err(anyhow::format_err!("key invalid: {:?}", other)),
        },
    ))
}

fn load_cert(cert_file: &Path) -> Result<rustls::Certificate, anyhow::Error> {
    let mut reader = BufReader::new(File::open(cert_file)?);
    let certs = rustls_pemfile::certs(&mut reader)?;
    if certs.is_empty() {
        return Err(anyhow::format_err!("no valid cert found in {cert_file:?}"));
    }
    Ok(rustls::Certificate(certs[0].clone()))
}

struct SelfSignedCertResolver {
    end_entity: rustls::Certificate,
}

impl ServerCertVerifier for SelfSignedCertResolver {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        if *end_entity == self.end_entity {
            println!("accepting self-signed cert");
            return Ok(rustls::client::ServerCertVerified::assertion());
        }
        Err(rustls::Error::General(
            "we accept only matching self-signed certs".into(),
        ))
    }
}
