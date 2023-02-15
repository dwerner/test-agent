
use std::net::IpAddr;
use std::sync::Arc;
use std::{marker::PhantomData, net::SocketAddr};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Future, Stream};
use futures::ready;
use pin_project::pin_project;
use rustls::ServerConfig;
use serde::{Deserialize, Serialize};
use tarpc::serde_transport::Transport as TarpcTransport;
use tarpc::tokio_serde::{Serializer, Deserializer};
use tokio_rustls::server::TlsStream;
use tokio_rustls::{TlsAcceptor};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::length_delimited;

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
    Transport(tarpc::serde_transport::new(framed_io, codec))
}

pub struct Transport<Item, SinkItem, Codec>(pub TarpcTransport<TlsStream<TcpStream>, Item, SinkItem, Codec>);

/// Listens on `addr`, wrapping accepted connections in TCP transports.
pub async fn listen<Item, SinkItem, Codec, CodecFn>(
    addr: &(IpAddr, u16),
    config: ServerConfig,
    codec_fn: CodecFn,
) -> io::Result<Incoming<Item, SinkItem, Codec, CodecFn>>
where
    Item: for<'de> Deserialize<'de>,
    Codec: Serializer<SinkItem> + Deserializer<Item>,
    CodecFn: Fn() -> Codec,
{
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    Ok(Incoming {
        acceptor,
        listener,
        codec_fn,
        local_addr,
        config: LengthDelimitedCodec::builder(),
        ghost: PhantomData,
    })
}

/// A [`TcpListener`] that wraps connections in [transports](Transport).
#[pin_project]
pub struct Incoming<Item, SinkItem, Codec, CodecFn> {
    acceptor: TlsAcceptor,
    listener: TcpListener,
    local_addr: SocketAddr,
    codec_fn: CodecFn,
    config: length_delimited::Builder,
    ghost: PhantomData<(fn() -> Item, fn(SinkItem), Codec)>,
}

impl<Item, SinkItem, Codec, CodecFn> Incoming<Item, SinkItem, Codec, CodecFn> {
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

impl<Item, SinkItem, Codec, CodecFn> Stream for Incoming<Item, SinkItem, Codec, CodecFn>
where
    Item: for<'de> Deserialize<'de>,
    SinkItem: Serialize,
    Codec: Serializer<SinkItem> + Deserializer<Item>,
    CodecFn: Fn() -> Codec,
{
    type Item = io::Result<Transport<Item, SinkItem, Codec>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let conn: TcpStream =
            ready!(Pin::new(&mut self.as_mut().project().listener).poll_accept(cx)?).0;
        let tls: TlsStream<TcpStream> = ready!(
            Pin::new(&mut self.acceptor.accept(conn)).poll(cx)?
        );

        Poll::Ready(Some(Ok(new(
            self.config.new_framed(tls),
            (self.codec_fn)(),
        ))))
    }
}
